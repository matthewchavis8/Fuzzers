#![allow(dead_code, unused_variables)]

use std::{num::NonZero, os::raw::c_uchar, path::PathBuf};

use libafl::{corpus::{InMemoryCorpus, InMemoryOnDiskCorpus}, events::SimpleEventManager, executors::{ExitKind, InProcessExecutor}, feedback_and_fast, feedbacks::{CrashFeedback, MaxMapFeedback, NewHashFeedback}, generators::RandPrintablesGenerator, inputs::{BytesInput, HasTargetBytes}, monitors::{SimpleMonitor, TuiMonitor}, mutators::{havoc_mutations, StdScheduledMutator}, observers::{self, BacktraceObserver, StdMapObserver}, schedulers::QueueScheduler, stages::StdMutationalStage, state::StdState, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, rands::StdRand, tuples::tuple_list, AsSlice};


unsafe extern "C" {
    static map_ptr: *mut u8;
    fn c_harness(input: *const c_uchar);
}

fn main() {
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buf = target.as_slice();

        unsafe { c_harness(buf.as_ptr()) };

        ExitKind::Ok
    };

    let observer = unsafe { StdMapObserver::from_mut_ptr("Observer", map_ptr, 8) };

    // StackTrace Observer
    let bt_observer = BacktraceObserver::owned("Stack Trace Observer", libafl::observers::HarnessType::InProcess);
    
    let mut feedback = MaxMapFeedback::new(&observer);
    let mut objective = feedback_and_fast!(CrashFeedback::new(), NewHashFeedback::new(&bt_observer));
    
    // Setting our state up
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryCorpus::new(), 
        InMemoryOnDiskCorpus::new(PathBuf::from("./solutions")).unwrap(), 
        &mut feedback, 
        &mut objective
    ).unwrap();

    #[cfg(feature = "tui")]
    let mon = TuiMonitor::builder()
        .title("Fuzzing C code InProcess")
        .enhanced_graphics(true)
        .build();

    #[cfg(not(feature = "tui"))]
    let mon = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));

    let mut mgr = SimpleEventManager::new(mon);
    
    let scheduler = QueueScheduler::new();
    
    // Setting up our fuzzer
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    // setting up executors
    let mut executor = InProcessExecutor::new(
        &mut harness, 
        tuple_list!(observer, bt_observer), 
        &mut fuzzer, 
        &mut state, 
        &mut mgr
    ).expect("Failed to create executor");

    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());

    state
        .generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, 8)
        .expect("Failed to load intial input");

    // creating stages
    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mutator_stage = StdMutationalStage::new(mutator);

    let mut stages = tuple_list!(mutator_stage);
    
    fuzzer
        .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
        .expect("Failed to start fuzz loop");
    


    
}
