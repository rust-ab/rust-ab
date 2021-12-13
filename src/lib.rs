pub mod engine;
pub mod utils;
pub use hashbrown;
pub use indicatif::ProgressBar;
pub use rand;
pub use core::fmt;

#[cfg(any(feature = "visualization", feature = "visualization_wasm", doc))]
pub mod visualization;

#[cfg(any(feature = "visualization", feature = "visualization_wasm", doc))]
pub use bevy;

pub use rand::{
    distributions::{Distribution, Uniform},
    thread_rng, Rng,
};

pub use csv::{Reader, Writer};
pub use rayon::prelude::*;
use std::error::Error;
pub use std::fs::File;
pub use std::fs::OpenOptions;
pub use std::io::Write;
pub use std::sync::{Arc, Mutex};
pub use std::time::Duration;

#[cfg(feature = "explore")]
pub use {
    memoffset::{offset_of, span_of},
    mpi::{datatype::UserDatatype, traits::*, Address},
    mpi::point_to_point as p2p,
    mpi::datatype::DynBufferMut,
    mpi::datatype::PartitionMut,
    mpi::Count,
};

#[cfg(feature = "explore")]
pub extern crate mpi;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum Info {
    Verbose,
    Normal,
}

/**
 * 3 mode to generate the data
 * Exaustive: Brute force parameter exploration
 * Matched: explore every input with the same indexes
 * File: Read from file
 */
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ExploreMode {
    Exaustive,
    Matched,
}

/**
 * 3 mode to do model exploration
 * Local: local computation
 * Parallel: parallel computation
 * Distributed: distributed computation
 */
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ComputationMode {
    Sequential,
    Parallel,
    DistributedMPI,
}

#[macro_export]
//step = simulation step number
//states
//# of repetitions
//type of info
macro_rules! simulate {
    ($step:expr, $s:expr, $reps:expr, $info:expr) => {{
        let mut s = $s;
        let mut state = s.as_state_mut();
        let n_step: u64 = $step;

        let mut results: Vec<(Duration, f32)> = Vec::new();
        let option = $info;

        match option {
            Info::Verbose => {
                println!("\u{1F980} Rust-AB v1.0\n");
                println!(
                    "{0: >10}|{1: >9}|    {2: >11}|{3: >10}|",
                    "#Rep", "Steps", "Steps/Seconds", "Time"
                );
                println!("--------------------------------------------------");
            }
            Info::Normal => {
                println!("{esc}c", esc = 27 as char);
                println!("\u{1F980} Rust-AB v1.0\n");
                println!(
                    "{0: >10}|{1: >9}|    {2: >11}|{3: >10}|",
                    "#Rep", "Steps", "Avg. Steps/Seconds", "Avg. Time"
                );
                println!("----------------------------------------------------------------");
            }
        }
        print!("{:width$}|", 1, width = 14 - $reps.to_string().len());
        println!(
            "{:width$}|",
            n_step,
            width = 15 - n_step.to_string().len() - $reps.to_string().len()
        );
        println!("{esc}c", esc = 27 as char);

        for r in 0..$reps {
            let mut schedule: Schedule = Schedule::new();
            state.init(&mut schedule);
            let start = std::time::Instant::now();
            let pb = ProgressBar::new(n_step);
            for i in 0..n_step {
                schedule.step(state);
                if state.end_condition(&mut schedule) {
                    break;
                }
                pb.inc(1);
            }
            pb.finish_with_message("\u{1F980}");

            let run_duration = start.elapsed();

            match option {
                Info::Verbose => {}
                Info::Normal => {
                    println!("{esc}c", esc = 27 as char);
                    println!("\u{1F980} Rust-AB v1.0\n");
                    println!(
                        "{0: >10}|{1: >9}|    {2: >11}|{3: >10}|",
                        "#Rep", "Steps", "Avg. Steps/Seconds", "Avg. Time"
                    );
                    println!("----------------------------------------------------------------");
                }
            }

            let step_seconds =
                format!("{:.0}", schedule.step as f32 / (run_duration.as_secs_f32()));
            let time = format!("{:.4}", run_duration.as_secs_f32());
            print!("{:width$}|", (r + 1), width = 14 - $reps.to_string().len());
            print!(
                "{:width$}|",
                schedule.step,
                width = 15 - n_step.to_string().len() - $reps.to_string().len()
            );
            print!("{:width$}", "", width = 13 - step_seconds.len());

            results.push((
                run_duration,
                schedule.step as f32 / (run_duration.as_nanos() as f32 * 1e-9),
            ));

            match option {
                Info::Verbose => {
                    print!("{}|", step_seconds);
                    print!("{:width$}", "", width = 9 - time.len());
                    println!("{}s|", time);
                }
                Info::Normal => {
                    let mut avg_time = 0.0;
                    let mut avg_step_seconds = 0.0;
                    for (time, step_seconds) in &results {
                        avg_time += time.as_secs_f32();
                        avg_step_seconds += step_seconds;
                    }
                    avg_time /= results.len() as f32;
                    avg_step_seconds /= results.len() as f32;
                    let avg_step_seconds = format!("{:.2}", avg_step_seconds);
                    let avg_time = format!("{:.4}", avg_time);
                    print!("{}|", avg_step_seconds);
                    print!("{:width$}", "", width = 9 - avg_time.len());
                    println!("{}s|", avg_time);
                }
            }
        }
        results
    }};
}

#[macro_use]
mod no_exported {
    #[macro_export]
    macro_rules! replace_expr {
        ($_t:tt $sub:expr) => {
            $sub
        };
    }

    //Used to count tokens of an expansion
    #[macro_export]
    macro_rules! count_tts {
        ($($tts:tt)*) => {<[()]>::len(&[$(replace_expr!($tts ())),*])};
    }

    #[macro_export]
    macro_rules! build_configurations{

        ($n_conf: expr, $( $input:ident )*) =>{{
        let mut config_table_index:Vec<Vec<usize>> = Vec::new();
        let mut input_size:usize = 0;
        let mut rep = $n_conf;
        {
            $(
                let mut row:Vec<usize> = Vec::with_capacity($n_conf);
                input_size = $input.len();
                rep /= input_size;
                let mut i = 0;
                for _ in 0..$n_conf{
                    for _ in 0..rep{
                            row.push(i);
                    }
                    i = (i + 1) % input_size;
                }
                config_table_index.push(row);
            )*
        }

        config_table_index
        }};

    }

    #[macro_export]
    macro_rules! simulate_explore {
        ($step:expr, $s:expr) => {{
            let mut s = $s;
            let mut state = s.as_state_mut();
            let n_step: u64 = $step;

            let mut results: Vec<(f32, f32)> = Vec::new();

            let mut schedule: Schedule = Schedule::new();
            state.init(&mut schedule);
            let start = std::time::Instant::now();

            for i in 0..n_step {
                schedule.step(state);

                if state.end_condition(&mut schedule) {
                    break;
                }
            }

            let run_duration = start.elapsed();

            results.push((
                run_duration.as_secs_f32(),
                schedule.step as f32 / (run_duration.as_nanos() as f32 * 1e-9),
            ));

            $s = s; // needed for model_exploration, requires also the state to be mut
            results
        }};
    }

    #[macro_export]
    ///step = simulation step number,
    ///schedule,
    ///states,
    ///input{input:type},
    ///output[output:type]
    macro_rules! explore_sequential {

        //exploration with explicit output parameters
        ($nstep: expr, $rep_conf:expr, $s:ty,
        input {$($input:ident: $input_ty: ty )*},
        output [$($output:ident: $output_ty: ty )*],
        $mode: expr,
        $( $x:expr ),* ) => {{

            //typecheck
            let _rep_conf = $rep_conf as usize;
            let _nstep = $nstep as u32;

            let mut n_conf:usize = 1;
            let mut config_table_index: Vec<Vec<usize>> = Vec::new();

            match $mode {
                ExploreMode::Exaustive =>{
                    $( n_conf *= $input.len(); )*
                    //Cartesian product with variadics, to build a table with all parameter combinations
                    //They are of different type, so i have to work with indexes
                    config_table_index = build_configurations!(n_conf, $($input )*);
                },
                ExploreMode::Matched =>{
                    $( n_conf = $input.len(); )*
                }
            }
            println!("n_conf {}", n_conf);

            let mut dataframe: Vec<FrameRow>  = Vec::new();


            for i in 0..n_conf{
                let mut state;
                // check which mode to use to generate the configurations
                match $mode {
                    // use all the possible combination
                    ExploreMode::Exaustive =>{
                        let mut row_count = -1.;
                        state = <$s>::new(
                            $(
                            $input[config_table_index[{row_count+=1.; row_count as usize}][i]],
                            )*
                        );
                    },
                    // create a configuration using the combination of input with the same index
                    ExploreMode::Matched =>{
                        state = <$s>::new(
                            $(
                                $input[i],
                            )*
                        );
                    }
                }

                println!("-----\nCONF {}", i);
                $(
                    println!("{}: {:?}", stringify!(state.$input), state.$input);
                )*

                for j in 0..$rep_conf{
                    println!("------\nRun {}", j+1);
                    let result = simulate_explore!($nstep, state);
                    dataframe.push(
                        FrameRow::new(i as u32, j + 1 as u32, $(state.$input,)* $(state.$output,)* result[0].0, result[0].1, $($x,)*)
                    );
                }
            }
            dataframe
        }};

        //exploration taking default output: total time and step per second
        ($nstep: expr, $rep_conf:expr, $s:expr, input {$($input:ident: $input_ty: ty )*}, $mode:expr) => {
            explore_sequential!($nstep, $s, $rep_conf, input {$($input: $input_ty)*}, output [], $mode)
        }

    }

    #[macro_export]
    macro_rules! explore_parallel {
        ($nstep: expr, $rep_conf:expr, $s:ty,
            input {$($input:ident: $input_ty: ty )*},
            output [$($output:ident: $output_ty: ty )*],
            $mode: expr,
            $( $x:expr ),* ) => {{

            //typecheck
            let _rep_conf = $rep_conf as usize;
            let _nstep = $nstep as u32;

            let mut n_conf:usize = 1;
            let mut config_table_index: Vec<Vec<usize>> = Vec::new();

            match $mode {
                ExploreMode::Exaustive =>{
                    $( n_conf *= $input.len(); )*
                    //Cartesian product with variadics, to build a table with all parameter combinations
                    //They are of different type, so i have to work with indexes
                    config_table_index = build_configurations!(n_conf, $($input )*);
                },
                ExploreMode::Matched =>{
                    $( n_conf = $input.len(); )*
                }
            }
            println!("n_conf {}", n_conf);

            let dataframe: Vec<FrameRow> = (0..n_conf*$rep_conf).into_par_iter().map( |run| {
                let i  = run / $rep_conf;

                let mut state;
                // check which mode to use to generate the configurations
                match $mode {
                    // use all the possible combination
                    ExploreMode::Exaustive =>{
                        let mut row_count = -1.;
                        state = <$s>::new(
                            $(
                            $input[config_table_index[{row_count+=1.; row_count as usize}][i]],
                            )*
                        );
                    },
                    // create a configuration using the combination of input with the same index
                    ExploreMode::Matched =>{
                        state = <$s>::new(
                            $(
                                $input[i],
                            )*
                        );
                    },
                }

                let result = simulate_explore!($nstep, state);
                FrameRow::new(i as u32, (run % $rep_conf) as u32, $(state.$input,)* $(state.$output,)* result[0].0, result[0].1, $($x,)*)
            })
            .collect();
            dataframe
        }};


        //exploration taking default output: total time and step per second
        ($nstep: expr, $rep_conf:expr, $state_name:ty, input {$($input:ident: $input_ty: ty )*,},
        $mode: expr) => {
                explore_parallel!($nstep, $rep_conf, $state_name, input { $($input: $input_ty)*}, output [],
                $mode)
        };
    }

    #[macro_export]
    macro_rules! explore_distributed_mpi {
        ($nstep: expr, $rep_conf:expr, $s:ty,
            input {$($input:ident: $input_ty: ty )*},
            output [$($output:ident: $output_ty: ty )*],
            $mode: expr,
            $( $x:expr ),* ) => {{

            // mpi initilization
            let universe = mpi::initialize().unwrap();
            let world = universe.world();
            let root_rank = 0;
            let root_process = world.process_at_rank(root_rank);
            let my_rank = world.rank();
            let num_procs = world.size() as usize;
            
            //typecheck
            let _rep_conf = $rep_conf as usize;
            let _nstep = $nstep as u32;

            let mut n_conf:usize = 1;
            let mut config_table_index: Vec<Vec<usize>> = Vec::new();

            // check which mode to use for the exploration
            match $mode {
                ExploreMode::Exaustive =>{
                    $( n_conf *= $input.len(); )*
                    //Cartesian product with variadics, to build a table with all parameter combinations
                    //They are of different type, so i have to work with indexes
                    config_table_index = build_configurations!(n_conf, $($input )*);
                },
                ExploreMode::Matched =>{
                    $( n_conf = $input.len(); )*
                },
            }
            println!("n_conf {}", n_conf/num_procs);

            let mut dataframe: Vec<FrameRow>  = Vec::new();
            for i in 0..n_conf/num_procs {
                let mut state;
                // check which mode to use to generate the configurations
                match $mode {
                    // use all the possible combination
                    ExploreMode::Exaustive =>{
                        let mut row_count = -1.;
                        state = <$s>::new(
                            $(
                            $input[config_table_index[{row_count+=1.; row_count as usize}][i*num_procs + (my_rank as usize)]],
                            )*
                        );
                    },
                    // create a configuration using the combination of input with the same index
                    ExploreMode::Matched =>{
                        state = <$s>::new(
                            $(
                                $input[i*num_procs + (my_rank as usize)],
                            )*
                        );
                    },
                }
                
                // execute the exploration for each configuration
                for j in 0..$rep_conf{
                    println!("conf {}, rep {}, pid: {}", i*num_procs + (my_rank as usize), j, my_rank);
                    let result = simulate_explore!($nstep, state);
                    dataframe.push(
                        FrameRow::new(i as u32, j + 1 as u32, $(state.$input,)* $(state.$output,)* result[0].0, result[0].1, $($x,)*)
                    );
                }
            }

            // must return a dummy dataframe that will not be used
            // since only the master write the complete dataframe of all procs on csv
            if world.rank() == root_rank {
                let mut all_dataframe = vec![dataframe[0]; n_conf];
                root_process.gather_into_root(&dataframe[..], &mut all_dataframe[..]);
                all_dataframe
            } else {
                //every proc send to root every row
                root_process.gather_into(&dataframe[..]);
                //return dummy dataframe
                dataframe = Vec::new();
                dataframe
            }
        }};

        //exploration taking default output: total time and step per second
        ($nstep: expr, $rep_conf:expr, $state_name:ty, input {$($input:ident: $input_ty: ty )*,},
        $mode: expr,
        $( $x:expr ),* ) => {
                explore_distributed_mpi!($nstep, $rep_conf, $state_name, input { $($input: $input_ty)*}, output [],
                $mode, $( $x:expr ),*)
        };
    }
}

#[macro_export]
//macro general to call exploration
macro_rules! explore {

    //exploration with explicit output parameters
    ($nstep: expr, $rep_conf:expr, $s:ty,
    input {$($input:ident: $input_ty: ty )*},
    output [$($output:ident: $output_ty: ty )*],
    $mode: expr,
    $cmode: expr,
    $( $x:ident: $x_ty: ty ),*
    ) => {{

        // optional parameters created for distributed mode
        $(
            // create a new variable for optional parameters and pass it as an optional expression
            let $x = $x;
        )*
        build_dataframe!(FrameRow, input {$( $input:$input_ty)* }, output[ $( $output:$output_ty )*], $( $x:$x_ty ),* );
        // check which computation mode is required for the exploration
        match $cmode {
            ComputationMode::Sequential => explore_sequential!(
                $nstep, $rep_conf, $s, input {$($input: $input_ty)*}, output [$($output: $output_ty)*], $mode, $( $x ),*
            ),
            ComputationMode::Parallel => explore_parallel!(
                $nstep, $rep_conf, $s, input {$($input: $input_ty)*}, output [$($output: $output_ty)*], $mode, $( $x ),*
            ),
            ComputationMode::DistributedMPI => explore_distributed_mpi!(
                $nstep, $rep_conf, $s, input {$($input: $input_ty)*}, output [$($output: $output_ty)*], $mode, $( $x ),*
            ),
        }
    }};

    ($nstep: expr, $rep_conf:expr, $state_name:ty, input {$($input:ident: $input_ty: ty )*,},
    $mode: expr,
    $cmode: expr) => {
                explore!($nstep, $rep_conf, $state_name, input { $($input: $input_ty)*}, output [],
                $mode, $cmode)
        };

}

///Create a csv file with the experiment results
///"DataFrame" trait allow the function to know field names and
///params list + output list for each configuration runned
pub fn write_csv<A: DataFrame>(name: &str, dataframe: &[A]) -> Result<(), Box<dyn Error>> {
    let csv_name = format!("{}.csv", name);
    let mut wtr = Writer::from_path(csv_name).unwrap();
    //define column name
    wtr.write_record(A::field_names())?;

    for row in dataframe {
        wtr.serialize(row.to_string())?;
    }

    Ok(())
}

///Trait implemented dynamically for our dataframe struct.
///Used into "export_dataframe" function
pub trait DataFrame {
    fn field_names() -> &'static [&'static str];
    fn to_string(&self) -> Vec<String>;
}

///Generate parameter values using a Uniform Distribution
///Params: Type, Min, Max and number of samples
///n_samples is optional, if omitted only a single simple is computed
#[macro_export]
macro_rules! gen_param {
    ( $type:ty, $min:expr, $max:expr, $n:expr) => {{
        let minimum: $type;
        let maximum: $type;
        minimum = $min;
        maximum = $max;
        let mut n = $n as usize;

        // Check parameters range to avoid error with Distribution
        let (minimum, maximum) = if minimum > maximum {
            (maximum, minimum)
        } else if minimum == maximum {
            (minimum, maximum + 1 as $type)
        } else {
            (minimum, maximum)
        };

        if n == 0 {
            n = 1;
        }

        let between = Uniform::from(minimum..maximum);
        let mut rng = rand::thread_rng();
        let dist: Vec<$type> = between.sample_iter(&mut rng).take($n).collect();

        dist
    }};

    // gen a single value
    (  $type:ty, $min:expr, $max:expr) => {{
        gen_param!($type, $min, $max, 1)
    }};
}

#[macro_export]
macro_rules! build_dataframe {
    //Dataframe with input and output parameters and optional parameters
    (
        $name:ident, input {$($input: ident: $input_ty: ty)*}, output [$($output: ident: $output_ty: ty)*], $( $x:ident: $x_ty: ty ),*
    ) => {

        #[derive(Default, Clone, Copy, PartialEq, Debug)]
        struct $name {
            pub conf_num: u32,
            pub conf_rep: u32,
            $(pub $input: $input_ty,)*
            $(pub $output: $output_ty,)*
            pub run_duration: f32,
            pub step_per_sec: f32,
            $(pub $x: $x_ty,)*
        }

        unsafe impl Equivalence for $name {
            type Out = UserDatatype;
            fn equivalent_datatype() -> Self::Out {

                //count input and output parameters to create slice for blocklen
                let v_in = count_tts!($($input)*);
                let v_out = count_tts!($($output)*);
                let v_x = count_tts!($($x)*);

                let dim = v_in + v_out + v_x + 4;
                let mut vec = Vec::with_capacity(dim);
                for i in 0..dim {
                    vec.push(1);
                }

                UserDatatype::structured(
                    vec.as_slice(),
                    &[
                        offset_of!($name, conf_num) as Address,
                        offset_of!($name, conf_rep) as Address,
                        $(
                            offset_of!($name, $input) as Address,
                        )*
                        $(
                            offset_of!($name, $output) as Address,
                        )*
                        offset_of!($name, run_duration) as Address,
                        offset_of!($name, step_per_sec) as Address,
                        $(
                            offset_of!($name, $x) as Address,
                        )*
                    ],
                    &[
                        u32::equivalent_datatype(),
                        u32::equivalent_datatype(),
                        $(
                            <$input_ty>::equivalent_datatype(),
                        )*
                        $(
                            <$output_ty>::equivalent_datatype(),
                        )*
                        f32::equivalent_datatype(),
                        f32::equivalent_datatype(),
                        $(
                            <$x_ty>::equivalent_datatype(),
                        )*
                    ]
                )
            }
        }

        impl DataFrame for $name{
            fn field_names() -> &'static [&'static str] {
                static NAMES: &'static [&'static str]
                    = &["Simulation", "Run", $(stringify!($input),)* $(stringify!($output),)*  "Run Duration", "Step per sec", $(stringify!($x),)*];
                NAMES
            }

            fn to_string(&self) -> Vec<String> {
                let mut v: Vec<String> = Vec::new();
                v.push(self.conf_num.to_string());
                v.push(self.conf_rep.to_string());
                $(
                    v.push(format!("{:?}", self.$input));
                )*
                $(
                    v.push(format!("{:?}", self.$output));
                )*
                v.push(self.run_duration.to_string());
                v.push(self.step_per_sec.to_string());
                $(
                    v.push(format!("{:?}", self.$x));
                )*
                v
            }

        }

        impl $name {
            pub fn new(
                conf_num: u32, conf_rep: u32 $(, $input: $input_ty)* $(, $output: $output_ty)*, run_duration: f32, step_per_sec: f32 $(, $x: $x_ty)*,
            ) -> $name{
                $name {
                    conf_num,
                    conf_rep,
                    $(
                        $input,
                    )*
                    $(
                        $output,
                    )*
                    run_duration,
                    step_per_sec,
                    $(
                        $x,
                    )*
                }
            }
        }
    };

    //Dataframe with only input parameters
    ($name:ident, input{$($element: ident: $input_ty: ty)* }) => {
        build_dataframe!($name, input{$($element: $input_ty)*}, output[]);
    };

    //Dataframe without output parameters
    ($name:ident, input {$($input: ident: $input_ty: ty)*}, $( $x:ident: $x_ty: ty ),*) => {
        build_dataframe!($name, input{$($element: $input_ty)*}, output[], $( $x:ident: $x_ty: ty ),*);
    };
}

#[macro_export]
macro_rules! load_csv {

    ($input_file: expr, $( $x:ident: $x_ty: ty ),*) =>{{
        let mut rdr = Reader::from_path($input_file).unwrap();
        $(
            let mut $x: Vec<$x_ty> = Vec::new();
        )*
        for result in rdr.records() {
            let record = result.unwrap();
            let mut i = 0;
            $(
                let x : $x_ty = record[i].parse().unwrap();
                $x.push(x);
                i += 1;
            )*
        }
        let v = ($( $x, )*);
        v
    }};
}

#[macro_export]
// genaral macro to perform model exploration using a genetic algorithm
// an individual is the state of the simulation to compute
// init_population: function that creates the population, must return an array of individual
// fitness: function that computes the fitness value, takes a single individual and the schedule, must return an f32
// mutation: function that perform the mutation, takes a single individual as parameter
// crossover: function that creates the population, takes the entire population as parameter
// state: state of the simulation representing an individual
// desired_fitness: desired fitness value
// generation_num: max number of generations to compute
// step: number of steps of the single simulation
// cmode: mode to perform the computation
// parameters(optional): parameter to write the csv, if not specified only fitness will be written
macro_rules! explore_ga {

    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
        $cmode: expr,
        parameters {
            $($p_name:ident: $p_type:ty)*
        }
    ) => {{

        build_dataframe_explore!(BufferGA, input {
            generation: u32
            index: i32
            fitness: f32
            $(
                $p_name: $p_type
            )*
        });

        let mut result: Option<Vec<_>> = None;
        match $cmode {
            ComputationMode::Sequential => {
                let r = explore_ga_sequential!(
                    $init_population,
                    $fitness,
                    $selection,
                    $mutation,
                    $crossover,
                    $state,
                    $desired_fitness,
                    $generation_num,
                    $step,
                    parameters {
                        $($p_name: $p_type)*
                    }
                );
                result = Some(r);
            },
            ComputationMode::Parallel => {
                let r = explore_ga_parallel!(
                    $init_population,
                    $fitness,
                    $selection,
                    $mutation,
                    $crossover,
                    $state,
                    $desired_fitness,
                    $generation_num,
                    $step,
                    parameters {
                        $($p_name: $p_type)*
                    }
                );
                result = Some(r);
            },
            ComputationMode::DistributedMPI => {
                let r = explore_ga_distributedMPI!(
                    $init_population,
                    $fitness,
                    $selection,
                    $mutation,
                    $crossover,
                    $state,
                    $desired_fitness,
                    $generation_num,
                    $step,
                    parameters {
                        $($p_name: $p_type)*
                    }
                );
                result = Some(r);
            },
        }
        result.unwrap() 
    }};

    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
        $cmode: expr
    ) => {
        explore_ga!( $init_population, $fitness, $selection, $mutation, $crossover, $state, $desired_fitness, $generation_num, $step, $cmode, parameters { }
        )
    };

}

// specific macro for explore_ga sequential
#[macro_export]
macro_rules! explore_ga_sequential {
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
        parameters {
            $($p_name:ident: $p_type:ty)*
        }
    ) => {{

        let mut generation = 0;
        let mut best_fitness = 0.;
        let mut best_generation = 0;

        let mut result:Vec<BufferGA> = Vec::new();

        // use init_population custom function to create a vector of state
        let mut population: Vec<$state> = $init_population();

        $(
            let mut $p_name: Option<$p_type> = None;
        )*

        // flag to break from the loop
        let mut flag = false;

        // calculate the fitness for the first population
        loop {
            // if generation_num is passed as 0, we have infinite generations
            if $generation_num != 0 && generation == $generation_num {
                println!("Reached {} generations, exiting...", $generation_num);
                break;
            }
            generation += 1;
            println!("Computing generation {}...", generation);

            let mut best_fitness_gen = 0.;
            // execute the simulation for each member of population
            // iterates through the population
            let mut index = 0;

            for individual in population.iter_mut() {
                // initialize the state
                let mut schedule: Schedule = Schedule::new();
                individual.init(&mut schedule);
                // compute the simulation
                for _ in 0..($step as usize) {
                    let individual = individual.as_state_mut();
                    schedule.step(individual);
                    if individual.end_condition(&mut schedule) {
                        break;
                    }
                }

                // compute the fitness value
                let fitness = $fitness(individual, schedule);

                // saving the best fitness of this generation
                if fitness >= best_fitness_gen {
                    best_fitness_gen = fitness;

                    $(
                        $p_name = Some(individual.$p_name);
                    )*
                }

                // result is here
                result.push(BufferGA::new(
                    generation,
                    index,
                    fitness,
                    $(
                        individual.$p_name,
                    )*
                ));

                // if the desired fitness is reached break
                // setting the flag at true
                if fitness >= $desired_fitness{
                    flag = true;
                    break;
                }
                index += 1;
            }

            // saving the best fitness of all generation computed until n
            if best_fitness_gen > best_fitness {
                best_fitness = best_fitness_gen;
                best_generation = generation;
            }

            println!("- best fitness in generation {} is {}", generation, best_fitness_gen);
            println!("-- best fitness is found in generation {} and is {}", best_generation, best_fitness);

            // if flag is true the desired fitness is found
            if flag {
                break;
            }

            // compute selection
            $selection(&mut population);
            // check if after selection the population size is too small
            if population.len() <= 1 {
                println!("Population size <= 1, exiting...");
                break;
            }

            // mutate the new population
            for individual in population.iter_mut() {
                $mutation(individual);
            }

            // crossover the new population
            $crossover(&mut population);
        }

        println!("The best individual has the following parameters ");
        $(
            println!("--- {} : {}", stringify!($p_name), $p_name.unwrap());
        )*
        println!("--- fitness : {}", best_fitness);
        result 
    }};

    // perform the model exploration with genetic algorithm without writing additional parameters
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
    ) => {
        explore_ga_sequential!( $init_population, $fitness, $selection, $mutation, $crossover, $state, $desired_fitness, $generation_num, $step, parameters { }
        );
    };
}

// specific macro for explore_ga in parallel using multiple processors
#[macro_export]
macro_rules! explore_ga_parallel {
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
        parameters {
            $($p_name:ident: $p_type:ty)*
        }
    ) => {{

        let mut generation = 0;
        let mut best_fitness = 0.;
        let mut best_generation = 0;

        // use init_population custom function to create a vector of state
        let mut population: Vec<$state> = $init_population();

        $(
            let mut $p_name: Option<$p_type> = None;
        )*

        // flag to break from the loop
        let mut flag = false;

        // Wrap the population into a Mutex to be safely shared
        let population = Arc::new(Mutex::new(population));
        let mut res: Vec<BufferGA> = Vec::new();

        // calculate the fitness for the first population
        loop {
            // if generation_num is passed as 0, we have infinite generations
            if $generation_num != 0 && generation == $generation_num {
                println!("Reached {} generations, exiting...", $generation_num);
                break;
            }
            generation += 1;
            println!("Computing generation {}...", generation);

            let mut best_fitness_gen = 0.;

            let len = population.lock().unwrap().len();
            
            let mut result = Vec::new();
            // execute the simulation for each member of population
            // iterates through the population

            (0..len).into_par_iter().map( |index| {
                // initialize the state
                let mut schedule: Schedule = Schedule::new();
                let mut individual: $state;
                {
                    let mut population = population.lock().unwrap();
                    // create the new state using the parameters
                    individual = <$state>::new(
                        $(
                            population[index].$p_name,
                        )*
                    );
                }

                // state initilization
                individual.init(&mut schedule);
                // simulation computation
                for _ in 0..($step as usize) {
                    let individual = individual.as_state_mut();
                    schedule.step(individual);
                    if individual.end_condition(&mut schedule) {
                        break;
                    }
                }

                // compute the fitness value
                let fitness = $fitness(&mut individual, schedule);
                
                BufferGA::new(
                    generation,
                    index as i32,
                    fitness,
                    $(
                        individual.$p_name,
                    )*
                )
                
                // return an array containing the results of the simulation to be written in the csv file
            }).collect_into_vec(&mut result);

            // for each simulation result
            for i in 0..result.len() {
                
                let fitness = result[i].fitness;
                population.lock().unwrap()[i].fitness = fitness;

                // saving the best fitness of this generation
                if fitness >= best_fitness_gen {
                    best_fitness_gen = fitness;
                    $(
                        $p_name = Some(population.lock().unwrap()[i].$p_name);
                    )*
                }

                // if the desired fitness set the flag at true
                if fitness >= $desired_fitness {
                    flag = true;
                }
            }

            // saving the best fitness of all generation computed until now
            if best_fitness_gen > best_fitness {
                best_fitness = best_fitness_gen;
                best_generation = generation;
            }

            println!("- best fitness in generation {} is {}", generation, best_fitness_gen);
            println!("-- best fitness is found in generation {} and is {}", best_generation, best_fitness);

            res.append(&mut result);
            
            // if flag is true the desired fitness is found
            if flag {
                break;
            }

            // compute selection
            $selection(&mut population.lock().unwrap());

            // check if after selection the population size is too small
            if population.lock().unwrap().len() <= 1 {
                println!("Population size <= 1, exiting...");
                break;
            }

            // mutate the new population
            for individual in population.lock().unwrap().iter_mut() {
                $mutation(individual);
            }

            // crossover the new population
            $crossover(&mut population.lock().unwrap());
        }

        println!("The best individual has the following parameters ");
        $(
            println!("--- {} : {}", stringify!($p_name), $p_name.unwrap());
        )*
        println!("--- fitness : {}", best_fitness);
        res
    }};

    // perform the model exploration with genetic algorithm without writing additional parameters
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
    ) => {
        explore_ga_parallel!( $init_population, $fitness, $selection, $mutation, $crossover, $state, $desired_fitness, $generation_num, $step, parameters { }
        );
    };
}

// specific macro for explore_ga using MPI for distributed computing
#[macro_export]
macro_rules! explore_ga_distributedMPI {
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
        parameters {
            $($p_name:ident: $p_type:ty)*
        }
    ) => {{

        // MPI initialization
        let universe = mpi::initialize().unwrap();
        let world = universe.world();
        let root_rank = 0;
        let root_process = world.process_at_rank(root_rank);
        let my_rank = world.rank();
        let num_procs = world.size() as usize;

        let mut generation: u32 = 0;
        let mut best_fitness = 0.;
        let mut best_generation = 0;
        let mut my_pop_size: usize = 0;
        let mut population: Vec<$state> = Vec::new();
        let mut population_size = 0;
      
        // create an array for each parameter
        $(
        let mut $p_name: Vec<$p_type> = Vec::new();
        )*

        let mut all_results: Vec<BufferGA> = Vec::new();
        
        let mut flag = false;
        
        // initialization of best individual placeholder
        let mut best_individual : Option<BufferGA> = None;

        if world.rank() == root_rank {
            population = $init_population();
            population_size = population.len();

            // dummy initilization
            best_individual = Some(BufferGA::new(
                0,
                0,
                0.,
                $(
                    population[0].$p_name,
                )*
            ));
        }
        
        loop {

            if $generation_num != 0 && generation == $generation_num {
                if world.rank() == root_rank {
                    println!("Reached {} generations, exiting...", $generation_num);
                }
                break;
            }
            generation += 1;
            let mut samples_count: Vec<Count> = Vec::new();
            // only the root process split the workload among the processes
            if world.rank() == root_rank {
                //create the whole population and send it to the other processes
                let mut population_size_per_process = population_size / num_procs;
      
                // for each processor
                for i in 0..num_procs {

                    let mut sub_population_size = 0;

                    if i == 0 {
                        sub_population_size = population_size - population_size_per_process * (num_procs - 1);
                    } else {
                        sub_population_size = population_size_per_process;
                    }        
                                
                    samples_count.push(sub_population_size as Count);
                    
                    // save my_pop_size for master 
                    if i == 0 {
                        my_pop_size = sub_population_size;
                    }
                    
                    // fulfill the parameters arrays
                    for j in 0..sub_population_size {
                        $(
                            $p_name.push(population[i * population_size_per_process + j].$p_name);
                        )*
                    }

                    // send the arrays
                    world.process_at_rank(i as i32).send(&sub_population_size);
                    $(
                        world.process_at_rank(i as i32).send(&$p_name[..]);
                    )*
                }
            } else {
                // every other processor receive the parameter
                let (my_population_size, _) = world.any_process().receive::<usize>();
                my_pop_size = my_population_size;
                $(
                    let (param, _) = world.any_process().receive_vec::<$p_type>();
                    $p_name = param;
                )*
            }

            let mut my_population: Vec<$state>  = Vec::new();

            for i in 0..my_pop_size {
                my_population.push(
                    <$state>::new(
                        $(
                            $p_name[i],  
                        )*
                    )
                );
            }
       
            if world.rank() == root_rank {
                println!("Computing generation {}...", generation);
            }

            let mut best_fitness_gen = 0.;

            let mut local_index = 0;

            // array collecting the results of each simulation run
            let mut my_results: Vec<BufferGA> = Vec::new();

            for individual in my_population.iter_mut() {
                // initialize the state
                let mut schedule: Schedule = Schedule::new();
                individual.init(&mut schedule);
                // compute the simulation
                for _ in 0..($step as usize) {
                    let individual = individual.as_state_mut();
                    schedule.step(individual);
                    if individual.end_condition(&mut schedule) {
                        break;
                    }
                }
                // compute the fitness value
                let fitness = $fitness(individual, schedule);
                
                // send the result of each iteration to the master
                
                let result = BufferGA::new(
                    generation,
                    local_index,
                    fitness,
                    $(
                        individual.$p_name,
                    )*
                );
                
                my_results.push(result);

                // saving the best fitness and the best individual of this generation
                // if fitness >= best_fitness_gen {
                //     best_fitness_gen = fitness;
                // }
                
                // if the desired fitness is reached break
                // setting the flag at true
                if fitness >= $desired_fitness{
                    flag = true;
                    break;
                }
                local_index += 1;
            }

            // receive simulations results from each processors
            if world.rank() == root_rank {

                let dummy = BufferGA::new(
                    generation,
                    0,
                    -999.,
                    $(
                        population[0].$p_name,
                    )*
                );

                let displs: Vec<Count> = samples_count
                    .iter()
                    .scan(0, |acc, &x| {
                        let tmp = *acc;
                        *acc += x;
                        Some(tmp)
                    })
                    .collect();

                let mut partial_results = vec![dummy; population_size];

                let mut partition = PartitionMut::new(&mut partial_results[..], samples_count.clone(), &displs[..]);
                // root receives all results from other processors
                root_process.gather_varcount_into_root(&my_results[..], &mut partition); 

                best_fitness_gen = 0.;
                // save the best individual of this generation
                let mut i = 0;
                let mut j = 0;
                for elem in partial_results.iter_mut() {
                    // only the master can update the index
                    elem.index += displs[i]; 
        
                    if elem.fitness > best_fitness_gen{
                        best_fitness_gen = elem.fitness;
                    }

                    if elem.fitness > best_individual.unwrap().fitness {
                        best_individual = Some(elem.clone());
                    }

                    j += 1;

                    if j == samples_count[i]{
                        i += 1;
                        j = 0;
                    }
    
                }
                
                // combine the results received
                all_results.append(&mut partial_results);
            } else {
                // send the result to the root processor
                root_process.gather_varcount_into(&my_results[..]);
            }

            // saving the best fitness of all generation computed until n
            if best_fitness_gen > best_fitness {
                best_fitness = best_fitness_gen;
                best_generation = generation;
            }

            if world.rank() == root_rank{
                println!("- best fitness in generation {} is {}", generation, best_fitness_gen);
                println!("-- best fitness is found in generation {} and is {}", best_generation, best_fitness);
            }

            // if flag is true the desired fitness is found
            if flag {
                break;
            }
            
            // the master do selection, mutation and crossover
            if world.rank() == root_rank {

                // set the population parameters owned by the master 
                // using the ones received from other processors
                for i in 0..population_size {
                    population[i].fitness = all_results[(generation as usize -1)*population_size + i].fitness;

                }

                // compute selection
                $selection(&mut population);

                // check if after selection the population size is too small
                if population.len() <= 1 {
                    println!("Population size <= 1, exiting...");
                    break;
                }

                // mutate the new population
                for individual in population.iter_mut() {
                    $mutation(individual);
                }

                // crossover the new population
                $crossover(&mut population);

                $(
                    $p_name.clear();
                )*
                
            }

        } // END OF LOOP
        
        if world.rank() == root_rank{
            println!("\nThe best individual has the following parameters ");
            println!("{:?}", best_individual.unwrap());   
        }

        // return arrays containing all the results of each simulation
        all_results
    }};

    // perform the model exploration with genetic algorithm without writing additional parameters
    (
        $init_population:tt,
        $fitness:tt,
        $selection:tt,
        $mutation:tt,
        $crossover:tt,
        $state: ty,
        $desired_fitness: expr,
        $generation_num: expr,
        $step: expr,
    ) => {
        explore_ga_distributedMPI!( $init_population, $fitness, $selection, $mutation, $crossover, $state, $desired_fitness, $generation_num, $step, parameters { }
        );
    };
}

#[macro_export]
macro_rules! build_dataframe_explore {
    //Dataframe with input and output parameters and optional parameters
    (
        $name:ident, input {$($input: ident: $input_ty: ty)*}
    ) => {

        #[derive(Default, Clone, Copy, PartialEq, Debug)]
        struct $name {
            
            $(pub $input: $input_ty,)*
            
        }

        unsafe impl Equivalence for $name {
            type Out = UserDatatype;
            fn equivalent_datatype() -> Self::Out {

                //count input and output parameters to create slice for blocklen
                let v_in = count_tts!($($input)*);
                

                let mut vec = Vec::with_capacity(v_in);
                for i in 0..v_in {
                    vec.push(1);
                }

                UserDatatype::structured(
                    vec.as_slice(),
                    &[
                        $(
                            offset_of!($name, $input) as Address,
                        )*
                    ],
                    &[
                        $(
                            <$input_ty>::equivalent_datatype(),
                        )*
                    ]
                )
            }
        }

        impl DataFrame for $name{
            fn field_names() -> &'static [&'static str] {
                static NAMES: &'static [&'static str]
                    = &[$(stringify!($input),)*];
                NAMES
            }

            fn to_string(&self) -> Vec<String> {
                let mut v: Vec<String> = Vec::new();
                $(
                    v.push(format!("{:?}", self.$input));
                )*
               
                v
            }

        }

        impl $name {
            pub fn new(
                $($input: $input_ty,)*
            ) -> $name{
                $name {
                   
                    $(
                        $input,
                    )*
            
                }
            }
        }

    };
}
