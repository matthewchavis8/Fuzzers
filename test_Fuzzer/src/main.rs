#![allow(dead_code, static_mut_refs)]

use std::{marker::PhantomData, num::NonZero, path::PathBuf, ptr::write};

use libafl::{corpus::{InMemoryCorpus, OnDiskCorpus}, events::SimpleEventManager, executors::{with_observers, Executor, ExitKind, WithObservers}, feedback_and_fast, feedbacks::{CrashFeedback, MaxMapFeedback}, generators::RandPrintablesGenerator, inputs::HasTargetBytes, monitors::{SimpleMonitor, TuiMonitor}, mutators::{havoc_mutations, ScheduledMutator, StdScheduledMutator}, observers::StdMapObserver, schedulers::QueueScheduler, stages::{AflStatsStage, CalibrationStage, MutationalStage, StdMutationalStage}, state::{HasCorpus, HasExecutions, StdState}, Fuzzer, StdFuzzer};
use libafl_bolts::{current_nanos, rands::StdRand, tuples::tuple_list, AsSlice};


// Creating a coverage Map
static mut SIGNALS: [u8; 8] = [0; 8];
// Creating a Ptr to Coverage Map
static mut SIGNALS_PTR: *mut u8 = unsafe { SIGNALS.as_mut_ptr() };

/*  This function marks our coverage map by flipping the 0 in the map to a 1
 *
 *  Parameters:
 *  @idx: the index into our coverage map
 *
 *  retunrs a void
 */
fn mark_signal(idx: usize) {
    unsafe { write(SIGNALS_PTR.add(idx), 1) };
}

struct CustomExecutor<S> {
    phantomdata: PhantomData<S>
}

impl <S> CustomExecutor<S> {
    fn new(_state: &S) -> Self {
        Self {
            phantomdata: PhantomData
        }
    }
}

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
        ) -> Result<libafl::executors::ExitKind, libafl::Error> 
    {
        *state.executions_mut() += 1;

        let target = input.target_bytes();
        let buff = target.as_slice();
        
        mark_signal(0);
        if !buff.is_empty() && buff[0] == b'M' {
            mark_signal(1);
            if buff.len() > 1 && buff[1] == b'A' {
                mark_signal(2);
                if buff.len() > 2 && buff[2] == b'T' {
                    mark_signal(3);
                    if buff.len() > 3 && buff[3] == b'T' {
                        mark_signal(4);
                        if buff.len() > 4 && buff[4] == b'H' {
                            mark_signal(5);
                            if buff.len() > 5 && buff[5] == b'E' {
                                mark_signal(6);
                                if buff.len() > 6 && buff[6] == b'W' {
                                    return Ok(ExitKind::Crash);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(ExitKind::Ok)
    }
}



fn main() {
    let map_observer = unsafe { StdMapObserver::from_mut_ptr("MapObserver", SIGNALS_PTR, SIGNALS.len()) };

    let mut feedback = MaxMapFeedback::new(&map_observer);
    let mut objective = feedback_and_fast!(
        CrashFeedback::new(),
        MaxMapFeedback::with_name("CrashObserver", &map_observer)
    );


    
    let calibration_stage = CalibrationStage::new(&feedback);
    let stats_stage = AflStatsStage::builder()
        .map_observer(&map_observer)
        .build()
        .unwrap();

    let mutator = StdScheduledMutator::new(havoc_mutations());
    let mutational_stage = StdMutationalStage::new(mutator);

    let mut scheduler = QueueScheduler::new();
    
    let mut state = StdState::new(
        StdRand::with_seed(current_nanos()), 
        InMemoryCorpus::new(), 
        OnDiskCorpus::new(PathBuf::from("./crashes")).unwrap(), 
        &mut feedback, 
        &mut objective
    ).unwrap();

    #[cfg(not(feature = "tui"))]
    let monitor = SimpleMonitor::new(|msg| println!("[LOG] {msg}"));

    #[cfg(feature = "tui")]
    let monitor = TuiMonitor::builder()
        .title("Test Fuzzer for messing around")
        .enhanced_graphics(false)
        .build();

    let mut mgr = SimpleEventManager::new(monitor);

    #[cfg(not(feature = "bloom_input_filter"))]
    let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);


    #[cfg(feature = "bloom_input_filter")]
    let mut fuzzer = StdFuzzer::with_bloom_input_filter(scheduler, feedback, objective, 1_000_000, 0.001);

    let mut generator = RandPrintablesGenerator::new(NonZero::new(32).unwrap());

    let executor = CustomExecutor::new(&state);
    let mut executor = WithObservers::new(executor, tuple_list!(map_observer));
    
    state
        .generate_initial_inputs(
            &mut fuzzer, 
            &mut executor, 
            &mut generator, 
            &mut mgr, 
            8
        )
        .unwrap();


    let mut stages = tuple_list!(
        calibration_stage,
        mutational_stage,
        stats_stage
    );

    fuzzer
        .fuzz_loop(&mut stages, &mut executor, &mut state, &mut mgr)
        .unwrap();
}

