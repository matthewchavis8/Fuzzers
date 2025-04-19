#![allow(static_mut_refs, dead_code)]

use std::{num::NonZero, path::PathBuf, ptr::write};

use libafl::{corpus::{Corpus, HasCurrentCorpusId, InMemoryCorpus, InMemoryOnDiskCorpus, OnDiskCorpus}, events::SimpleEventManager, executors::{ExitKind, InProcessExecutor}, feedbacks::{CrashFeedback, MaxMapFeedback}, generators::RandPrintablesGenerator, inputs::{BytesInput, HasTargetBytes}, monitors::SimpleMonitor, mutators::{havoc_mutations, ScheduledMutator, StdScheduledMutator}, observers::StdMapObserver, schedulers::QueueScheduler, stages::{ObserverEqualityFactory, StagesTuple, StdMutationalStage, StdTMinMutationalStage}, state::{HasCorpus, HasSolutions, StdState}, Error, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, rands::StdRand, tuples::tuple_list, AsSlice};

// Creating a Coverage Map for instrumentation
static mut SIGNALS: [u8; 16] = [0; 16];
// Creating Ptr to Coverage Map
static mut SIGNALS_PTR: *mut u8 =  unsafe { SIGNALS.as_mut_ptr() };
// Global Variable for our Coverage map length
static mut SIGNALS_LEN: usize = unsafe { SIGNALS.len() };

/* This function marks our coverage map and flips the 0->1 
 * marking that new coverage or coverage has been found
 *
 * @param idx The index of our coverage map we will mark as covered
 *
 * return void This does not return anything instead just flipping the 0->1 marking as covered
 */
fn mark_mp(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1) };
}


fn main() -> Result<(), Error> {
    // This lambda is how we will link our target fuzzer with our executor
    let mut harness = |input: &BytesInput| {
        let target = input.target_bytes();
        let buff = target.as_slice();
        
        mark_mp(0);
        if !buff.is_empty() && buff[0] == b'M' {
            mark_mp(1);
            if buff.len() > 1 && buff[1] == b'A' {
                mark_mp(2);
                if buff.len() > 2 && buff[2] == b'T' {
                    return ExitKind::Crash;
                }
            }
        }
        ExitKind::Ok
    };

    let observer = unsafe { StdMapObserver::from_mut_ptr("MapObserver", SIGNALS_PTR, SIGNALS_LEN) };
    
    let factory = ObserverEqualityFactory::new(&observer);

    let mut feedback = MaxMapFeedback::new(&observer);
    let mut objective = CrashFeedback::new();

    let mon = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));

    let mut mgr = SimpleEventManager::new(mon);

    let corpus_dr = PathBuf::from("./corpus");
    let crash_dir = PathBuf::from("./crashes");

    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryOnDiskCorpus::new(corpus_dr).unwrap(), 
        OnDiskCorpus::new(&crash_dir).unwrap(), 
        &mut feedback, 
        &mut objective
    )
    .unwrap();

    let scheduler = QueueScheduler::new();

    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    let mut executor = InProcessExecutor::new(
        &mut harness, 
        tuple_list!(observer), 
        &mut fuzzer, 
        &mut state, 
        &mut mgr
    )
    .expect("Failed to create the Executor");

    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());

    state
        .generate_initial_inputs(&mut fuzzer, &mut executor, &mut generator, &mut mgr, 8)
        .expect("Failed to generate the initial corpus");
    

    // Setting up mutational stage
    let mutator = StdScheduledMutator::new(havoc_mutations());
    let  mutator_stage = StdMutationalStage::new(mutator);
    // Setting up a minimizer stage
    let minimizer = StdScheduledMutator::new(havoc_mutations());
    let  minimizer_stage = StdTMinMutationalStage::new(minimizer, factory, 128);
    let mut stages = tuple_list!(
            mutator_stage,
            minimizer_stage
        );
    // Basically we keep on fuzzing until a crash has been found 
    while state.solutions().is_empty() {
        fuzzer.fuzz_one(&mut stages, &mut executor, &mut state, &mut mgr).unwrap();
    } 
    
    /*
     * Once we found our crash we will take that crash and try minimizing
     * while still keeping the case a solution 
     *
     * Note we also create a second fuzzer instance to fuzz the minimize case
     * tldr reduce the size
     *
     */
    let minimized_dir = PathBuf::from("./minimized");
    
    // In this state we are going to store and muatate our test case 
    // starting from our first solution constantly mutating it and storing it on OnDiskCorpus
    // This is why we are keeping our Solution corpus in memory because we are trying to reduce
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryOnDiskCorpus::new(minimized_dir).unwrap(), 
        InMemoryCorpus::new(), 
        &mut (), 
        &mut ()
    )
    .unwrap();

    let mon = SimpleMonitor::new(|msg| println!("[LOG] {}", msg));

    let mut mgr = SimpleEventManager::new(mon);

    let minimizer = StdScheduledMutator::new(havoc_mutations());
    
    // This minMutationalStage is going to keep reducing our corpus entry by 1024 bytes 
    // It will keep on reducing as long as the test case is still interesting
    let mut stages = tuple_list!(
        StdTMinMutationalStage::new(minimizer, CrashFeedback::new(), 1 << 10)
    );

    let scheduler = QueueScheduler::new();
    
    // We are reducing so no need to include feedback or objective as nothing will be added
    let mut fuzzer = StdFuzzer::new(scheduler, (), ());
    
    // We are not using an observer so you can leave it blank
    let mut executor = InProcessExecutor::new(&mut harness, (), &mut fuzzer, &mut state, &mut mgr).unwrap();
   
    // Here we are loading our corpus with the solution we will minimize
    state.load_initial_inputs_forced(&mut fuzzer, &mut executor, &mut mgr, &[crash_dir]).unwrap();
    
    // Grabs the Crash ID loaded in the first corpus aka InMemoryOnDiskCorpus
    let first_id = state.corpus().first().expect("Empty Corpus");
    // Sets the ID we grabbed as the first test case we want to run
    state.set_corpus_id(first_id).expect("failed to set first ID");
    
    // Executres all stages on our solution test case in order to reduce it as much as possible
    // In the end returns the most minimized Successsful test case
    stages.perform_all(&mut fuzzer, &mut executor, &mut state, &mut mgr).unwrap();

    Ok(())
}

