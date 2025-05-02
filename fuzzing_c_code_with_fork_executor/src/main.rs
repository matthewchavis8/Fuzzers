#![allow(dead_code, unused_variables)]

// Note: So you can call C or C++ methods in Rust using extern
// However you have to redefine it in a rust way 
// Example: 
// unsafe extern "C" {
//     fn foo();
//     fn add(x: i32, y: i32) -> i32;
// }

use std::{num::NonZero, path::PathBuf, process::Child, time::Duration};

use libafl::{corpus::{InMemoryCorpus, InMemoryOnDiskCorpus}, events::SimpleEventManager, executors::{ExitKind, InProcessForkExecutor}, feedback_and_fast, feedbacks::{CrashFeedback, MaxMapFeedback, NewHashFeedback}, generators::RandPrintablesGenerator, inputs::{BytesInput, HasTargetBytes}, monitors::{SimpleMonitor, TuiMonitor}, mutators::{havoc_mutations, StdScheduledMutator}, observers::{BacktraceObserver, StdMapObserver}, schedulers::QueueScheduler, stages::StdMutationalStage, state::StdState, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, ownedref::OwnedRefMut, rands::StdRand, shmem::{ShMemProvider, StdShMemProvider}, tuples::tuple_list, AsSlice};
use libc::{c_int, c_uchar};


unsafe extern "C" {
    fn c_harness(input: *const c_uchar);
    fn create_shmem_array() -> c_int;
    fn get_ptr() -> *mut u8;
}

fn main() {
    let mut shmem_provider = StdShMemProvider::new().unwrap(); 
    unsafe { create_shmem_array() };

    let mp_ptr = unsafe { get_ptr() };

    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buf = target.as_slice();

        unsafe { c_harness(buf.as_ptr()) }

        ExitKind::Ok
    };
    
    // Regular Observer
    let observer = unsafe {
        StdMapObserver::from_mut_ptr("Signals", mp_ptr, 8)
    };

    // backtrace observer
    let mut bt = shmem_provider.new_on_shmem::<Option<u64>>(None).unwrap();

    let bt_observer = BacktraceObserver::new(
        "BacktraceObserver", 
        unsafe { OwnedRefMut::from_shmem(&mut bt) }, 
        libafl::observers::HarnessType::Child
    );

    let mut feedback = MaxMapFeedback::new(&observer);
    
    // This objective is checking for a crash and if we discover a new backtrace
    let mut objective = feedback_and_fast!(
        CrashFeedback::new(),
        NewHashFeedback::new(&bt_observer)
    );

    // Creating a state
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryCorpus::new(), 
        InMemoryOnDiskCorpus::new(PathBuf::from("./solutions")).unwrap(),
        &mut feedback, 
        &mut objective
    ).expect("Failed to create state");

    #[cfg(feature = "tui")] 
    let mon = TuiMonitor::builder()
        .title("Fuzzing a C code with backtrace")
        .enhanced_graphics(true)
        .build();

    #[cfg(not(feature = "tui"))]
    let mon = SimpleMonitor::new(|msg| println!("[LOG]: {msg}"));

    let mut mgr = SimpleEventManager::new(mon);

    // Schedule test cases
    let scheduler = QueueScheduler::new();

    // Creating the Fuzzer
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    // Creating the executor for an in-process function with just one observer
    let mut executor = InProcessForkExecutor::new(
        &mut harness, 
        tuple_list!(observer, bt_observer), 
        &mut fuzzer, 
        &mut state, 
        &mut mgr, 
        Duration::from_millis(5000), 
        shmem_provider
    ).expect("Failed to start executor");
    
    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());
    state
        .generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, 8)
        .expect("Failed to generate intial corpus");

    // Setting up stages
    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mutator_stage = StdMutationalStage::new(mutator);

    let mut stages = tuple_list!(mutator_stage);

    fuzzer
        .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
        .expect("Failed to start fuzz loop");



}
