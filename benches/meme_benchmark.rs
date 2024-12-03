// Copyright 2024 Junshuang Hu
mod measurements;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
//use pprof::criterion::{Output, PProfProfiler};

use measurements::RxxOPS;
use std::{thread, time::Duration};

use meme::{
    core::{
        IMem, PObj, RequestedObj,
    }, helpers, mems::basic::BasicMem, objs::com::{ObjChannel, SendMsg, SendWrapper}, rules::{
        com::SendReceiveRule, BasicCondition, BasicEffect
    }
};
use meme::meme_derive::{IObj, IRule};

#[derive(IObj, IRule, Debug)]
pub struct BenchRuleAct {
    #[tag]
    t: u32,
    #[condition]
    cond: BasicCondition<u32>,
    #[effect]
    eff: BasicEffect<u32>
}

impl BenchRuleAct {
    pub fn new(tag: u32, rtgd_amount: usize) -> Self {
        Self {
            t: tag,

            cond: helpers::condition_builder()
            .rand_tagged::<BenchO>(rtgd_amount).by_tag()
            .build(),

            eff: helpers::effect_builder()
            .remove_obj(|req: &mut RequestedObj<u32>| {
                req.rand_tags(0).unwrap().get(0).unwrap().clone()
            })
            .crate_obj(|_| Box::new(BenchO::new(helpers::IdGen::next_u32_id())))
            //.crate_obj(|_| Box::new(StopObj { tag: helpers::IdGen::next_u32_id() }))
            .increase_untagged::<StopObj>(1)
            .build(),
        }
    }
}

#[derive(IObj, Debug)]
pub struct StopObj {
    #[tag]
    tag: u32
}

#[derive(IObj, IRule, Debug, Clone)]
pub struct BenchStopRule {
    #[tag]
    tag: u32,
    #[effect]
    eff: BasicEffect<u32>,
    #[condition]
    cond: BasicCondition<u32>
}

impl BenchStopRule {
    pub fn new(tag: u32, stop_amount: u32) -> Self {
        Self {
            tag,

            cond: helpers::condition_builder()
                .some_untagged::<StopObj>(stop_amount)
                .build(),

            eff: helpers::effect_builder()
                .stop_mem()
                .build(),
        }
    }
}

#[derive(IObj, IRule, Debug)]
pub struct BenchComRule {
    #[tag]
    tag: u32,
    #[effect]
    eff: BasicEffect<u32>,
    #[condition]
    cond: BasicCondition<u32>
}

impl BenchComRule {
    pub fn new(tag: u32, to_ch: u32) -> Self {
        Self {
            tag,
            cond: helpers::condition_builder()
                .the_tagged(to_ch).by_ref()
                .build(),

            eff: helpers::effect_builder()
                .crate_obj(|req| {
                    let co = req.set_ref(0).unwrap();
                    let ct: &u32 = co.obj_tag();
                    let v = vec![
                        SendWrapper::new(Box::new(BenchO::new(helpers::IdGen::next_u32_id())), ct.clone())
                    ];
                    let pobj = Box::new(SendMsg::<u32>::new(helpers::IdGen::next_u32_id(), v));
                    pobj
                })
                .build(),
        }
    }
}

#[derive(Debug, IObj)]
struct BenchO {
    #[tag]
    tg: u32
}
impl BenchO {
    pub fn new(tg: u32) -> Self {
        Self { tg: tg }
    }
}

fn criterion_benchmark(c: &mut Criterion<RxxOPS>) {
    let elements: [u32; 7] = [1_000, 5_000, 10_000, 50_000, 100_000, 500_000, 1_000_000];
    let est_time: [u64; 7] = [8, 15, 30, 125, 300, 3000, 6000];
    let name= ["R-TOPS", "R-5-TOPS", "R-10-TOPS", "R-50-TOPS" ,"R-HTOPS", "R-5-THOPS", "R-MOPS"];
    let loop_count = 1_000_u32;
    let select_amount = 100;

    let mut group = c.benchmark_group("throughput-rxxops");
    for ((elems, name), e_time) in elements.iter().zip(name.iter()).zip(est_time.iter()) {
        group.throughput(Throughput::Elements(loop_count as u64));
        group.measurement_time(Duration::from_secs(*e_time));
        group.bench_with_input(format!("{name}"), elems, |b,elems| {
            let obj_count = *elems as usize;
            let (ta, tb) = (helpers::IdGen::next_u32_id(), helpers::IdGen::next_u32_id());
            b.iter_batched_ref( 
                || {
                    let mut m = BasicMem::<u32, u32>::new(0, false);
                    let mut os: Vec<PObj<u32>> = Vec::with_capacity(obj_count + 1);
                    os.resize_with(obj_count, || Box::new(BenchO::new(helpers::IdGen::next_u32_id())));
                    let (ca, cb) = ObjChannel::<u32>::new_pair(ta, tb);
                    os.push(Box::new(ca));
                    m.init(
                        os,
                        vec![
                            Box::new(BenchRuleAct::new(helpers::IdGen::next_u32_id(), select_amount)),
                            Box::new(BenchStopRule::new(helpers::IdGen::next_u32_id(), loop_count)),
                            Box::new(BenchComRule::new(helpers::IdGen::next_u32_id(), ta)),
                            Box::new(SendReceiveRule::new(helpers::IdGen::next_u32_id(), vec![ta])),
                        ]
                    );
                    let listen_handel = thread::spawn(move || {
                        let mut count = 0;
                        while count < loop_count {
                            let _ = cb.receive();
                            count += 1;
                        }
                    });
                    (m, Some(listen_handel))
                },
                |(m, listen_handel)| {
                    let _ = m.start();
                    let _ = listen_handel.take().unwrap().join();
                },
                BatchSize::SmallInput
            )
        });
    }
    group.finish();

}

criterion_group!{
    name = meme_indexes;
    
    config = Criterion::default()
    //.with_profiler(PProfProfiler::new(100, Output::Flamegraph(Some(pprof::flamegraph::Options::default()))))
    .with_measurement(RxxOPS);

    targets = criterion_benchmark
}
criterion_main!(meme_indexes);