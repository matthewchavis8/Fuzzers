#![allow(unused_variables)]
// A fuzzer using qemu in systemmode for binary only coverage of kernels

use std::{env, path::PathBuf, time::Duration};

use libafl::{executors::ExitKind, inputs::BytesInput, observers::{CanTrack, HitcountsMapObserver, TimeObserver, VarLenMapObserver, VariableMapObserver}};
use libafl_bolts::{core_affinity::Cores, ownedref::OwnedMutSlice};
use libafl_qemu::{breakpoint::Breakpoint, command::{EndCommand, StartCommand}, elf::EasyElf, modules::StdEdgeCoverageModule, Emulator, GuestAddr, GuestPhysAddr, GuestReg, QemuMemoryChunk};
use libafl_targets::{edges_map_mut_ptr, EDGES_MAP_DEFAULT_SIZE, MAX_EDGES_FOUND};

pub static mut MAX_INPUT_SIZE: usize = 50;

pub fn fuzz() {
    env_logger::init();

    // if let Ok(s) = env::var("FUZZ_SIZE") {
    //     str::parse::<usize>(&s).expect("FUZZ_SIZE was not a number");
    // }
    
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
            emulator.run(state, input).expect("Failed to execute QEMU");
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

    };

    println!("Successfully BUILT");
}
