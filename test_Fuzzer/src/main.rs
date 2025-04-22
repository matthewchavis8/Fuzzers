#![allow(static_mut_refs, dead_code)]

use std::{num::NonZero, path::PathBuf, ptr::write};

use libafl::{corpus::{Corpus, HasCurrentCorpusId, InMemoryCorpus, InMemoryOnDiskCorpus, OnDiskCorpus}, events::SimpleEventManager, executors::{ExitKind, InProcessExecutor}, feedbacks::{CrashFeedback, MaxMapFeedback}, generators::RandPrintablesGenerator, inputs::{BytesInput, HasTargetBytes}, monitors::SimpleMonitor, mutators::{havoc_crossover, havoc_mutations, StdScheduledMutator}, observers::StdMapObserver, schedulers::QueueScheduler, stages::{ObserverEqualityFactory, StagesTuple, StdMutationalStage, StdTMinMutationalStage}, state::{HasCorpus, HasSolutions, StdState}, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, rands::StdRand, tuples::{tuple_list, Prepend}, AsSlice};


static mut SIGNALS: [u8; 16] = [0; 16];
static mut SIGNALS_PTR: *mut u8 = unsafe { SIGNALS.as_mut_ptr() };

fn mark_mp(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1) };
}

fn main() -> Result<(), ExitKind> {
   
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buff = target.as_slice();
        
        mark_mp(0);
        if !buff.is_empty() && buff[0] == b'M' {
            mark_mp(1);
            if buff.len() > 1 && buff[1] == b'A' {
                mark_mp(2);
                if buff.len() > 2 && buff[2] == b'T' {
                    mark_mp(3);
                    if buff.len() > 3 && buff[3] == b'T' {
                        return ExitKind::Crash;
                    }
                } 
            }
        }

        ExitKind::Ok
    };

    let observer = unsafe { StdMapObserver::from_mut_ptr("Map Observer", SIGNALS_PTR, SIGNALS.len()) };

    let mut feedback = MaxMapFeedback::new(&observer);
    let mut objective = CrashFeedback::new();
        
    
    let factory = ObserverEqualityFactory::new(&observer);

    let monitor = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));

    let mut mgr = SimpleEventManager::new(monitor);
    
    let corpus_dir = PathBuf::from("./corpus");
    let solution_dir = PathBuf::from("./solution");

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryOnDiskCorpus::new(corpus_dir).unwrap(), 
        OnDiskCorpus::new(&solution_dir).unwrap(), 
        &mut feedback, 
        &mut objective
    ).expect("Failed to create state");
    
    let scheduler = QueueScheduler::new();

    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());
    
    let mut executor = InProcessExecutor::new(
        &mut harness, 
        tuple_list!(observer), 
        &mut fuzzer, 
        &mut state,
        &mut mgr)
        .expect("Failed creating executioner");

    state
        .generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, 8)
        .expect("Failed to load intial state");
    
    // Setting up a mutator stage and minimizer stage for our entries
    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mutate_stage = StdMutationalStage::new(mutator);

    let minimizer = StdScheduledMutator::new(havoc_mutations());
    let minimizer_stage = StdTMinMutationalStage::new(minimizer, factory, 128);
    
    let mut stages = tuple_list!(
        mutate_stage,
        minimizer_stage
    );

    while state.solutions().is_empty() {
        fuzzer
            .fuzz_one(&mut stages, &mut executor, &mut state, &mut mgr).unwrap();
    }

    // OK so now if we get our solution corpus
    // when we find a successful crash we will now minimizing the successful crash
    // reason being it reduces size and makes things easier to read


    let monitor = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));
    let mut mgr = SimpleEventManager::new(monitor);

    let minimized_dir = PathBuf::from("./minimized");
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryOnDiskCorpus::new(minimized_dir).unwrap(), 
        InMemoryCorpus::new(),
        &mut (), 
        &mut ()
    )
    .expect("Failed to create state");

    let scheduler = QueueScheduler::new();
    let mut fuzzer = StdFuzzer::new(scheduler, (), ());

    let mut executor = InProcessExecutor::new(
        &mut harness, 
        (), 
        &mut fuzzer, 
        &mut state, 
        &mut mgr
    ).unwrap();

    let minimizer = StdScheduledMutator::new(havoc_mutations());
    let minimizer_stage = StdTMinMutationalStage::new(minimizer, CrashFeedback::new(), 1 << 10);
    
    let mut stages = tuple_list!(
        minimizer_stage
    );

    state
        .load_initial_inputs_forced(&mut fuzzer, &mut executor, &mut mgr, &[solution_dir])
        .unwrap();

    let first_id = state.corpus().first().expect("Failed to grab the first crash");
    state.set_corpus_id(first_id).expect("Failed to set the first id");

    stages
        .perform_all(&mut fuzzer, &mut executor, &mut state, &mut mgr)
        .unwrap();




    Ok(())
}
