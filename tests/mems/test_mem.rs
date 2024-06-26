use std::{thread, time::Duration};

use meme::{core::*, mems::base_mem::BaseMem, objs::base_objs::{ExampleObj, ExampleRule}};
use uuid::Uuid;

#[test]
pub fn basics() {
    let (s, _r) = crossbeam_channel::unbounded();

    let mut m = BaseMem::<Uuid, i32>::new(s, Uuid::new_v4());
    let mut obj_a = Box::new(ExampleObj::<Uuid, i32>::new(Uuid::new_v4()));
    let mut obj_b = Box::new(ExampleObj::<Uuid, i32>::new(Uuid::new_v4()));
    let rule_a = Box::new(ExampleRule::<Uuid, i32>::new(Uuid::new_v4()));
    obj_a.data_push_val(666);
    obj_a.data_push_val(777);
    obj_a.data_push_val(111);
   
    obj_b.data_push_val(999);
    obj_b.data_push_val(666);
    obj_b.data_push_val(111);

    let m_id = m.get_id();

    let ini_res = m.init();
    assert!(ini_res.is_ok());
    let s = ini_res.unwrap();

    let handel = thread::spawn(move || {
        let r = m.start();
        (r, m)
    });

    assert!(s.send(Operation { op_type: OperationType::ObjAdd, target_id: m_id, data: MsgDataObj::Obj(obj_a) }).is_ok());
    assert!(s.send(Operation { op_type: OperationType::ObjAdd, target_id: m_id, data: MsgDataObj::Obj(obj_b) }).is_ok());
    assert!(s.send(Operation { op_type: OperationType::RuleAdd, target_id: m_id, data: MsgDataObj::Rule(rule_a) }).is_ok());
    thread::sleep(Duration::from_secs(10));
    assert!(s.send(Operation { op_type: OperationType::Stop, target_id: Default::default(), data: MsgDataObj::None }).is_ok());

    let res = handel.join();
    assert!(res.is_ok());

    let (m_res, m_new) = res.unwrap();
    assert!(m_res.is_ok());
    assert_eq!(true, m_res.unwrap());

    let pr_obj = m_new.get_pref_objs();
    assert!(pr_obj.values().all(|o| {
        o.get_ref_data_vec().contains(&666) &&
        o.get_ref_data_vec().contains(&222) &&
        o.get_ref_data_vec().contains(&3552) 
    }));
}
