use std::{thread, time::Duration};

use meme::{core::*, mems::base_mem::BaseMem, objs::base_objs::{BaseObj, TestRuleA, TestRuleB}};

#[test]
pub fn basics() {
    let (s, _r) = crossbeam_channel::unbounded();

    let mut m = BaseMem::<i32>::new(s);
    let rule_a = Box::new(TestRuleA::<i32>::new());
    let rule_b = Box::new(TestRuleB::<i32>::new());
    let mut obj_a = Box::new(BaseObj::<i32>::new());
    obj_a.push_data(666);
    obj_a.push_data(777);

    let ini_res = m.init();
    assert!(ini_res.is_ok());
    let s = ini_res.unwrap();

    let handel = thread::spawn(move || {
        let r = m.start();
        (r, m)
    });

    assert!(s.send(Operation { op_type: OperationType::RuleAdd, target_id: Default::default(), data: MsgDataObj::Rule(rule_a) }).is_ok());
    assert!(s.send(Operation { op_type: OperationType::RuleAdd, target_id: Default::default(), data: MsgDataObj::Rule(rule_b) }).is_ok());
    assert!(s.send(Operation { op_type: OperationType::ObjAdd, target_id: obj_a.get_id(), data: MsgDataObj::Obj(obj_a) }).is_ok());

    thread::sleep(Duration::from_secs(3));
    
    assert!(s.send(Operation { op_type: OperationType::Stop, target_id: Default::default(), data: MsgDataObj::None }).is_ok());

    let res = handel.join();

    assert!(res.is_ok());
    let (m_res, m_new) = res.unwrap();
    assert!(m_res.is_ok());
    assert_eq!(true, m_res.unwrap());
    for o in m_new.get_pref_objs() {
        println!("id: {},  obj val: {:?}", o.0, o.1.get_ref_data_vec());
    }

}
