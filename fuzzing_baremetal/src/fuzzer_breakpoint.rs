#![allow(unused_variables)]
// A fuzzer using qemu in systemmode for binary only coverage of kernels

use std::{env, path::PathBuf, time::Duration};

use libafl_bolts::core_affinity::Cores;
use libafl_qemu::{elf::EasyElf, GuestAddr};

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
    
    /* 
     *  Creates a growable byte buffer that contains the binary of the elf file
     *
     *  This buffer is used to parse our f
     * */
    let mut elf_buffer = Vec::new();
    let elf = EasyElf::from_file(
        env::var("KERNEL").expect("KERNEL env not set"),
        &mut elf_buffer,
    )
    .unwrap();
    
    // Memory Address to the input buffer where our mutated testcases will get injected in
    let input_addr = elf
        .resolve_symbol(
            &env::var("FUZZ_INPUT").unwrap_or_else(|_| "FUZZ_INPUT".to_owned()), 
            0
        )
        .expect("env FUZZ_INPUT not found or having trouble finding the input buffer in binary") as GuestAddr;

    // Memory Address to the main function in our harness where coverage begins
    let main_addr = elf
        .resolve_symbol(
            &env::var("Main").unwrap_or_else(|_| "Main".to_owned()), 
            0
        )
        .expect("env Main not set or having trouble finding main function in binary");

    // Memory Address to the breakpoint where coverage should end
    let breakpoint_addr = elf
        .resolve_symbol(
            &env::var("BREAKPOINT").unwrap_or_else(|_| "BREAKPOINT".to_owned()), 
            0
        )
        .expect("env BREAKPOINT not set or having trouble finding BREAKPOINT in binary");



    println!("Successfully BUILT");
}
