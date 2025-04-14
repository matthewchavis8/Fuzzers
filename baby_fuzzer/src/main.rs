#![allow(static_mut_refs, dead_code)]

use std::{num::NonZero, path::PathBuf, ptr::write};

use libafl::{corpus::{InMemoryCorpus, OnDiskCorpus}, events::SimpleEventManager, executors::{ExitKind, InProcessExecutor}, feedbacks::{CrashFeedback, MaxMapFeedback}, generators::RandPrintablesGenerator, inputs::{BytesInput, HasTargetBytes}, monitors::{SimpleMonitor, TuiMonitor}, mutators::{havoc_mutations, StdScheduledMutator}, observers::StdMapObserver, schedulers::QueueScheduler, stages::StdMutationalStage, state::StdState, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, rands::StdRand, tuples::tuple_list};

static mut SIGNALS: [u8; 8] = [0; 8];
static mut SIGNALS_PTR: *mut u8 = unsafe {
    SIGNALS.as_mut_ptr()
};
fn mark_signal(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1)};
}

fn main() {
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buf = target.iter().as_slice();
        
        mark_signal(0);
        if !buf.is_empty() && buf[0] == b'M' {
            mark_signal(1);
            if buf.len() > 1 && buf[1] == b'A' {
                mark_signal(2);
                if buf.len() > 2 && buf[2] == b'T' {
                    mark_signal(3);
                    if buf.len() > 3 && buf[3] == b'T' {
                        panic!("Found the objective");
                    }
                }
            }
        }
        ExitKind::Ok
    };

    let observer =  unsafe { StdMapObserver::from_mut_ptr("MapObserver", SIGNALS_PTR, SIGNALS.len()) };
    let mut feedback = MaxMapFeedback::new(&observer);
    let mut objective = CrashFeedback::new(); 

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()),
        InMemoryCorpus::new(),
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(), 
        &mut feedback,
        &mut objective
        )
        .expect("Error setting up state");

    let scheduler = QueueScheduler::new();

    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);
    
    #[cfg(not(feature = "tui"))]
    let monitor = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));
    
    #[cfg(feature = "tui")]
    let monitor = TuiMonitor::builder()
       .title("Baby Fuzzer")
       .enhanced_graphics(false)
       .build();
    
    let mut event_mgr = SimpleEventManager::new(monitor);
    let mut executor = InProcessExecutor::new(
        &mut harness, 
        tuple_list!(observer), 
        &mut fuzzer, 
        &mut state, 
        &mut event_mgr
    ).unwrap();
    
    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());

    state
        .generate_initial_inputs(
            &mut fuzzer, 
            &mut executor, 
            &mut generator,
            &mut event_mgr, 
            8
        )
        .unwrap();

    let mutator = StdScheduledMutator::new(havoc_mutations());

    let mut stages = tuple_list!(StdMutationalStage::new(mutator));

    fuzzer
        .fuzz_loop( 
            &mut stages, 
            &mut executor,
            &mut state,
            &mut event_mgr
            )
        .unwrap();
}
