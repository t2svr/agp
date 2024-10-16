use meme::core::IndexMap;

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

