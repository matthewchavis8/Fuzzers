#![allow(unused_variables)]
use std::{env, num::NonZero, path::PathBuf, time::Duration};
use libafl::{
        corpus::{InMemoryCorpus, OnDiskCorpus}, 
        events::{EventConfig, Launcher}, feedback_or, 
        feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback, TimeoutFeedback}, generators::RandPrintablesGenerator, 
        inputs::BytesInput, monitors::{MultiMonitor, TuiMonitor}, mutators::{havoc_mutations, StdScheduledMutator}, 
        observers::{CanTrack, HitcountsMapObserver, TimeObserver, VariableMapObserver}, 
        schedulers::{IndexesLenTimeMinimizerScheduler, QueueScheduler}, stages::{CalibrationStage, StdMutationalStage}, 
        state::StdState, Error, Fuzzer, StdFuzzer};

use libafl_bolts::{core_affinity::Cores, current_nanos, ownedref::OwnedMutSlice, rands::StdRand, 
                shmem::{ ShMemProvider, StdShMemProvider}, tuples::tuple_list};
use libafl_qemu::{modules::StdEdgeCoverageModule, Emulator, QemuExecutor};
use libafl_targets::{edges_map_mut_ptr, EDGES_MAP_DEFAULT_SIZE, MAX_EDGES_FOUND};

pub fn fuzz() {
    /*
     * Hard coded Parameters
     *
     * @var timeout     - maximum time a test case can run before timing out
     * @var broker_port - broker process for all fuzzer instances to connect to and coordinate together 
     * @var cores       - assigning worker processes to core '1'
     * @var corpus_dir  - Interesting testcases are placed here 
     * @var crash_dir   - successful testcases are placed here
     * */
    let timeout = Duration::from_secs(3);
    let broker_port = 1337;
    let cores = Cores::from_cmdline("1").unwrap();
    let corpus_dir = [PathBuf::from("./corpus")];
    let crash_dir = PathBuf::from("./crashes");
    
    /*
     * After broker is set up the qemu launcher will invoke to the client process once
     * Basically each processes main function 
     *
     * @param state              - if restarting a worker it carries over else fresh start
     * @param mgr                - event manager built with multimonitor to handle reporting with the broker
     * @param client_description - meta data that must be linked
     * */
    let mut run_client = |state: Option<_>, mut mgr, _client_description| {
        let args: Vec<String> = env::args().collect();

        // Harness calling the LLVM-style harness
        let mut harness = |
            emulator: &mut Emulator<_,_,_,_,_,_,_,>,
            state: &mut _,
            input: &BytesInput| unsafe {
                emulator.run(state, input).unwrap().try_into().unwrap()
            };
        
        // Created an observeration channel to watch code coverage
        let mut edges_observer = unsafe {
            HitcountsMapObserver::new(VariableMapObserver::from_mut_slice(
                    "edges", 
                    OwnedMutSlice::from_raw_parts_mut(edges_map_mut_ptr(), EDGES_MAP_DEFAULT_SIZE), 
                    &raw mut MAX_EDGES_FOUND,
            ))
            .track_indices()
        };
        
        // Created an observation channel to keep track of execution time
        let time_observer = TimeObserver::new("Time");

        // Initialize QEMU Emulator
        let emu = Emulator::builder()
            .qemu_parameters(args)
            .prepend_module(
                StdEdgeCoverageModule::builder()
                .map_observer(edges_observer.as_mut())
                .build()
                .expect("Failed to intialize coverage map in QEMU"),
            )
            .build()
            .expect("Failed to call QEMU emulator");
        
        // todo: would like to move these print statments into a LOG file instead hmmmm
        #[cfg(not(feature = "tui"))]
        let devices = emu.list_devices();
        #[cfg(not(feature = "tui"))]
        println!("Devices: {:?}", devices);

        // Feedback to rate the interestingness of an input
        // Can eitheir be a slower executions or a new coverage
        let mut feedback = feedback_or!(
            MaxMapFeedback::new(&edges_observer),
            TimeFeedback::new(&time_observer), 
        );

        // Objective to rate what is a solution
        // A solution can eitheir be a timout or a crash
        let mut objective = feedback_or!(
            CrashFeedback::new(),
            TimeoutFeedback::new()
        );

        // If not restarting state, create a state from scratch
        let mut state = state.unwrap_or_else(|| {
            StdState::new(
                StdRand::with_seed(current_nanos()), 
                InMemoryCorpus::new(), 
                OnDiskCorpus::new(&crash_dir).unwrap(), 
                &mut feedback, 
                &mut objective
            )
            .expect("Failed to create state")
        });

        // A minimization + queue policy to grab testcases from the corpus
        let scheduler =
            IndexesLenTimeMinimizerScheduler::new(&edges_observer, QueueScheduler::new());

        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        // Creating a mutational stage and a calibration stage
        let mutator = StdScheduledMutator::new(havoc_mutations());
        let calibration_feedback = MaxMapFeedback::new(&edges_observer);

        let mut stages = tuple_list!(
            StdMutationalStage::new(mutator),
            CalibrationStage::new(&calibration_feedback)
        );

        // Intializing the QEMU in-process executor
        let mut executor = QemuExecutor::new(
            emu, 
            &mut harness, 
            tuple_list!(edges_observer, time_observer), 
            &mut fuzzer, 
            &mut state, 
            &mut mgr, 
            timeout
        )
        .expect("Failed to start QEMU executor");
        
        // trigger a breakpoint
        executor.break_on_timeout();
        
        if state.must_load_initial_inputs() {
            let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());
            let test_cases = 8;
            state.generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, test_cases)
                    .expect("Failed to load empty corpus with intial input");

            println!("[LOG] Loaded {test_cases} testcases into corpus");
        }

        fuzzer
            .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
            .unwrap();

        Ok(())
    };

    // Shared Memory allocator so processes can communicate with eachother
    let shmem_provider = StdShMemProvider::new().expect("Failed to init shared memory");

    // Stats reporter for the broker
    #[cfg(not(feature = "tui"))]
    let monitor = MultiMonitor::new(|msg| println!("[LOG] {msg}"));
    
    #[cfg(feature = "tui")]
    let monitor = TuiMonitor::builder()
        .enhanced_graphics(true)
        .title("Fuzzing Baremetal ARM with sync_exit")
        .build();

    // Build and run launcher
    #[cfg(not(feature = "tui"))]
    match Launcher::builder()
        .shmem_provider(shmem_provider)
        .broker_port(broker_port)
        .configuration(EventConfig::from_build_id())
        .monitor(monitor)
        .run_client(&mut run_client)
        .cores(&cores)
        .build()
        .launch()
        {
            Ok(()) => (),
            Err(Error::ShuttingDown) => println!("User stopped fuzzing process"),
            Err(e) => panic!("Failed to run launcher: {e:?}"),
        }
    
    // if tui is enabled fuzzer output would cover it so moving it to an external file
    #[cfg(feature = "tui")]
    match Launcher::builder()
        .shmem_provider(shmem_provider)
        .broker_port(broker_port)
        .configuration(EventConfig::from_build_id())
        .monitor(monitor)
        .run_client(&mut run_client)
        .cores(&cores)
        .stdout_file(Some("/dev/null"))
        .stderr_file(Some("/dev/null"))
        .build()
        .launch()
        {
            Ok(()) => (),
            Err(Error::ShuttingDown) => println!("User stopped fuzzing process"),
            Err(e) => panic!("Failed to run launcher: {e:?}"),
        }
}
