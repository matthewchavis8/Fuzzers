#![allow(unused_variables)]
use std::{env, num::NonZero, path::PathBuf, process, time::Duration};
use libafl::{
        corpus::{Corpus, InMemoryCorpus, OnDiskCorpus}, 
        events::{EventConfig, Launcher}, executors::ExitKind, feedback_or, 
        feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback, TimeoutFeedback}, generators::RandPrintablesGenerator, 
        inputs::BytesInput, monitors::{MultiMonitor, TuiMonitor}, mutators::{havoc_mutations, StdScheduledMutator}, 
        observers::{CanTrack, HitcountsMapObserver, TimeObserver, VariableMapObserver}, 
        schedulers::{IndexesLenTimeMinimizerScheduler, QueueScheduler}, stages::{CalibrationStage, StdMutationalStage}, 
        state::{HasCorpus, StdState}, Error, Fuzzer, StdFuzzer};

use libafl_bolts::{core_affinity::Cores, current_nanos, ownedref::OwnedMutSlice, rands::StdRand, 
                shmem::{ShMemProvider, StdShMemProvider}, tuples::tuple_list};
use libafl_qemu::{breakpoint::Breakpoint, command::{EndCommand, StartCommand}, 
                  elf::EasyElf, modules::StdEdgeCoverageModule, Emulator, GuestPhysAddr, GuestReg, QemuExecutor, QemuMemoryChunk};
use libafl_targets::{edges_map_mut_ptr, EDGES_MAP_DEFAULT_SIZE, MAX_EDGES_FOUND};

pub static mut MAX_INPUT_SIZE: usize = 50;

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
    
    // Creates a growable byte buffer that contains the binary of the elf file
    let mut elf_buffer = Vec::new();
    let elf = EasyElf::from_file(
        env::var("KERNEL").expect("KERNEL env not set"),
        &mut elf_buffer,
    )
    .unwrap();
    
    // Memory Address to the input buffer where our mutated testcases will get injected into QEMU
    let input_addr = elf
        .resolve_symbol(
            &env::var("FUZZ_INPUT").unwrap_or_else(|_| "FUZZ_INPUT".to_owned()), 
            0
        )
        .expect("env FUZZ_INPUT not found or having trouble finding the input buffer in binary") as GuestPhysAddr;
    println!("input address: {:#X}", input_addr);

    // Memory Address to the main function in our harness where coverage begins
    let main_addr = elf
        .resolve_symbol(
            &env::var("main").unwrap_or_else(|_| "main".to_owned()), 
            0
        )
        .expect("env Main not set or having trouble finding main function in binary");
    println!("main address: {:#X}", main_addr);

    // Memory Address to the breakpoint where coverage should end
    let breakpoint_addr = elf
        .resolve_symbol(
            &env::var("BREAKPOINT").unwrap_or_else(|_| "BREAKPOINT".to_owned()), 
            0
        )
        .expect("env BREAKPOINT not set or having trouble finding BREAKPOINT in binary");
    println!("Break point address: {:#X}", breakpoint_addr);
    
    /*
     * After broker is set up the qemu launcher will invoke to the client process once
     * Basically each processes main function 
     *
     * @param state              - if restarting a worker it carries over else fresh start
     * @param mgr                - event manager built with multimonitor to handle reporting with the broker
     * @param client_description -
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

        // Set the start point for QEMU
        emu.add_breakpoint(
            Breakpoint::with_command(
                main_addr, 
                StartCommand::new(QemuMemoryChunk::phys(
                        input_addr, 
                        unsafe { MAX_INPUT_SIZE } as GuestReg, 
                        None,
                ))
                .into(),
                true
            ),
            true
        );
        
        // Set the end point for QEMU
        emu.add_breakpoint(
            Breakpoint::with_command(
                breakpoint_addr, 
                EndCommand::new(Some(ExitKind::Ok)).into(), 
                false
            ), 
            true
        );

        let devices = emu.list_devices();
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

            println!("[LOG] Loaded {test_cases} into corpus");
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
        .title("Fuzzing Baremetal ARM")
        .build();

    // Build and run launcher
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

    println!("Successfully BUILT");
}
