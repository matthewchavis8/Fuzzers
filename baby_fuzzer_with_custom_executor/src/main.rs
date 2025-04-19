#![allow(dead_code, static_mut_refs)]

use std::{marker::PhantomData, num::NonZero, path::PathBuf, ptr::write};

use libafl::{corpus::{InMemoryCorpus, OnDiskCorpus}, events::SimpleEventManager, executors::{Executor, ExitKind, WithObservers}, feedback_and_fast, feedbacks::{CrashFeedback, MaxMapFeedback}, generators::RandPrintablesGenerator, inputs::HasTargetBytes, monitors::{SimpleMonitor, TuiMonitor}, mutators::{havoc_mutations, StdScheduledMutator}, observers::StdMapObserver, schedulers::{QueueScheduler, StdScheduler}, stages::{push::StdMutationalPushStage, AflStatsStage, CalibrationStage, StdMutationalStage}, state::{HasCorpus, HasExecutions, StdState}, BloomInputFilter, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, rands::StdRand, tuples::tuple_list, AsSlice};

// Creating the coverage map
static mut SIGNALS: [u8; 8] = [0; 8];
// Ptr to coverage map
static mut SIGNALS_PTR: *mut u8 = unsafe { SIGNALS.as_mut_ptr() };

// Function to mark if new coverage is found in the fuzzer
fn mark_map(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1) };
}

// Creating a Custom exectutor and linking it to the state
struct CustomExecutor<S> {
    phantomdata: PhantomData<S>
}

// Creating a constructor for our Custom Executor
impl <S> CustomExecutor<S> {
    pub fn new(_state: &S) -> Self {
        Self {
            phantomdata: PhantomData
        }
    }
}

// implementing the executor trait for our Custom Executor
impl <EM, I, S, Z> Executor<EM, I, S, Z> for CustomExecutor<S> 
where 
    S: HasCorpus<I> + HasExecutions,
    I: HasTargetBytes
{
    fn run_target(
            &mut self,
            _fuzzer: &mut Z,
            state: &mut S,
            _mgr: &mut EM,
            input: &I,
        ) -> Result<libafl::executors::ExitKind, libafl::Error> {
        *state.executions_mut() += 1;
    
        let target = input.target_bytes();
        let buff = target.as_slice();
        mark_map(0);
       if !buff.is_empty() && buff[0] == b'M' {
           mark_map(1);
           if buff.len() > 1 && buff[1] == b'A' {
               mark_map(2); 
               if buff.len() > 2 && buff[2] == b'T' {
                    mark_map(3);
                   if buff.len() > 3 && buff[3] == b'T' {
                       println!("[LOG]: {}", String::from_utf8_lossy(buff));
                        return Ok(ExitKind::Crash)
                    }
                }
           }
       }
       Ok(ExitKind::Ok)  
    }
}

fn main() {

    // Creating our coverage map
    let observer = unsafe { StdMapObserver::from_mut_ptr("MapObserver", SIGNALS_PTR, SIGNALS.len()) };
    
    // Creating our feedback to add interesting test cases into the corpus
    let mut feedback = MaxMapFeedback::new(&observer);

    // Creating our objective and adding another observer for only UNIQUE crashes
    let mut objective = feedback_and_fast!(
       MaxMapFeedback::with_name("Crash", &observer),
       CrashFeedback::new(),
    );
    

    // This stage checks the stability of our test
    let calibration_stage = CalibrationStage::new(&feedback);
    
    // This plots data points onto our graph
    let stats_change = AflStatsStage::builder()
        .map_observer(&observer)
        .build()
        .unwrap();
    

    // Creating our fuzzer state that will contain all of our metadata
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryCorpus::new(), 
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(), 
        &mut feedback, 
        &mut objective
    )
        .expect("error creating state");

    #[cfg(not(feature = "tui"))]
    let monitor = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));

    #[cfg(feature = "tui")]
    let monitor = TuiMonitor::builder()
        .title("Baby Fuzzer with CustomExecutor")
        .enhanced_graphics(false)
        .build();

    let mut event_manager = SimpleEventManager::new(monitor);
    
    let scheduler = QueueScheduler::new();
    
    
    #[cfg(not(feature = "bloom_input_filter"))]
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

    #[cfg(feature = "bloom_input_filter")]
    let mut fuzzer = StdFuzzer::with_bloom_input_filter(scheduler, feedback, objective, 1_000_000, 0.001);
    
    let executor = CustomExecutor::new(&state);
    let mut executor = WithObservers::new(executor, tuple_list!(observer));

    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());
    
    state
        .generate_initial_inputs(
            &mut fuzzer, 
            &mut executor, 
            &mut generator, 
            &mut event_manager, 
            8
        ).unwrap();
        
    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mut stages = tuple_list!(
        calibration_stage,
        StdMutationalStage::new(mutator),
        stats_change
    );

    fuzzer
        .fuzz_loop(&mut stages, &mut executor, &mut state, &mut event_manager)
        .unwrap();
}
