use std::borrow::Borrow;

use meme::core::IndexMap;
use rand::{thread_rng, Rng};

#[test]
pub fn index_map_test() {
    let mut m = IndexMap::new();
    m.insert(1, 'a');
    m.insert(2, 'b');
    m.insert(3, 'c');
    m.insert(4, 'd');
    assert_eq!(*m.get(&1).unwrap(), 'a');
    assert_eq!(m.index_of(&2).unwrap(), 1);
    let v1 = vec!['a', 'b', 'c', 'd'];
    assert!(v1.iter().zip(m.vals().iter()).all(|(a, b)| {
        a == b
    }));
    let r = m.get_mut(&4).unwrap();
    *r = 'e';
    assert_eq!(*m.get(&4).unwrap(), 'e');
    assert_eq!(m.remove(&3).unwrap(), (3, 'c'));
    let v2 = vec!['a',  'b',  'e'];
    assert!(v2.iter().zip(m.vals().iter()).all(|(a, b)| {
        a == b
    }));
    assert_eq!(m.index_of(&1), Some(0));
}

#[test]
pub fn choose_iter_test() {
    let mut rng = thread_rng();
    let branch = rng.gen_bool(0.5);
    let data = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
    let rule_indexes = if branch { Some(vec![0, 1, 3, 4, 5, 2]) } else { None };
    let all_ind;
    let mut ite = if let Some(ri) = rule_indexes.borrow() {
        ri
    } else {
        all_ind = (0..data.len()).collect::<Vec<_>>();
        &all_ind
    }.iter().filter_map(|i| data.get(*i));
    assert_eq!(ite.next(), Some(&0.0));
    if branch { 
        assert_eq!(ite.next(), Some(&0.1));
        assert_eq!(ite.next(), Some(&0.3));
        assert_eq!(ite.next(), Some(&0.4));
    } else {
        assert_eq!(ite.next(), Some(&0.1));
        assert_eq!(ite.next(), Some(&0.2));
        assert_eq!(ite.next(), Some(&0.3));
    }
  
}