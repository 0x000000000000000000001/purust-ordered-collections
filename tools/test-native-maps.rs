#![allow(warnings)]
// Differential test for the native `Data.Map.Internal` primitives of
// `purust-ordered-collections`.
//
// The generated workspace contains both lanes:
//   - the PureScript oracles `Data_Map_Internal_{insert,insertWith,unionWith}PS`
//     (explicit comparator, no `Ord` dictionary), used here as the reference;
//   - the native exports `Data_Map_Internal_{insert,insertWith,unionWith}Impl`
//     (oracle first argument, then the same explicit comparator), appended from
//     `src/Data/Map/Internal.rs`.
// The fixture compares them directly, so a candidate source that embeds the
// native exports is exactly what must be tested.
use purust_core::*;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::Arc as Rc;
use Purs_Data_Map_Internal::{
    Data_Map_Internal_insertImpl, Data_Map_Internal_insertPS,
    Data_Map_Internal_insertWithImpl, Data_Map_Internal_insertWithPS,
    Data_Map_Internal_unionWithImpl, Data_Map_Internal_unionWithPS,
    Map,
};
use Purs_Data_Maybe::Maybe;
use Purs_PureScript_Backend_Optimizer_CoreFn::Qualified;

fn snapshot(map: &Map) -> String {
    match map {
        Map::Leaf => String::from("_"),
        Map::Node(h, s, k, v, l, r) => format!("({h},{s},{},{},{},{})", key(k), v.unwrap_int(), snapshot(l), snapshot(r)),
    }
}

fn key(value: &Value) -> String {
    match value.resolve() {
        Value::Int(n) => n.to_string(),
        Value::String(s) => format!("{s:?}"),
        _ => {
            #[cfg(purust_class_shared)]
            let owner = value.unwrap_class_shared::<Qualified>();
            #[cfg(not(purust_class_shared))]
            let owner = value.unwrap_class::<Rc<Qualified>>().clone();
            let Qualified::Qualified(m, i) = owner.as_ref();
            format!("{}:{}", match m.as_ref() { Maybe::Nothing => "-".to_string(), Maybe::Just(v) => key(v) }, key(i))
        }
    }
}

// Legacy nested representation: `Value::Class(Rc<Rc<Qualified>>)`.
fn qualified_legacy(module: Option<&str>, ident: &str) -> Value {
    Value::Class(Rc::new(Rc::new(Qualified::Qualified(Rc::new(match module {
        None => Maybe::Nothing, Some(s) => Maybe::Just(Value::String(purust_string_from_utf8(s)))
    }), Value::String(purust_string_from_utf8(ident))))))
}

// Shared-owner representation: `Value::ClassShared(Rc<Qualified>)`.
#[cfg(purust_class_shared)]
fn qualified_shared(module: Option<&str>, ident: &str) -> Value {
    Value::ClassShared(Rc::new(Qualified::Qualified(Rc::new(match module {
        None => Maybe::Nothing, Some(s) => Maybe::Just(Value::String(purust_string_from_utf8(s)))
    }), Value::String(purust_string_from_utf8(ident)))))
}

fn qualified_keys() -> Vec<Value> {
    let mut out = Vec::new();
    for module in [None, Some(""), Some("A"), Some("B"), Some("é")] {
        for ident in ["", "x", "y", "🦀"] {
            out.push(qualified_legacy(module, ident));
            // Under the shared-owner runtime both representations denote the
            // same key and must compare equal through the oracle callback.
            #[cfg(purust_class_shared)]
            {
                out.push(qualified_shared(module, ident));
            }
        }
    }
    out
}

type Compare = Func2<Value, Value, Purs_Data_Ordering::Ordering>;
type Combine = Func2<Value, Value, Value>;

fn dummy_insert_oracle() -> Func4<Compare, Value, Value, Rc<Map>, Rc<Map>> {
    Func4::Static(|_, _, _, _| Rc::new(Map::Leaf))
}
fn dummy_insert_with_oracle() -> Func5<Compare, Combine, Value, Value, Rc<Map>, Rc<Map>> {
    Func5::Static(|_, _, _, _, _| Rc::new(Map::Leaf))
}
fn dummy_union_oracle() -> Func4<Compare, Combine, Rc<Map>, Rc<Map>, Rc<Map>> {
    Func4::Static(|_, _, _, _| Rc::new(Map::Leaf))
}

fn counted_compare(counter: Rc<AtomicUsize>, descending: bool) -> Compare {
    Func2::Shared(Rc::new(move |a: Value, b: Value| {
        counter.fetch_add(1, AtomicOrdering::Relaxed);
        let ordering = if descending { b.unwrap_int().cmp(&a.unwrap_int()) } else { a.unwrap_int().cmp(&b.unwrap_int()) };
        match ordering {
            std::cmp::Ordering::Less => Purs_Data_Ordering::Ordering::LT,
            std::cmp::Ordering::Equal => Purs_Data_Ordering::Ordering::EQ,
            std::cmp::Ordering::Greater => Purs_Data_Ordering::Ordering::GT,
        }
    }))
}

fn counted_combine(counter: Rc<AtomicUsize>) -> Combine {
    Func2::Shared(Rc::new(move |old: Value, new: Value| {
        counter.fetch_add(1, AtomicOrdering::Relaxed);
        Value::Int(old.unwrap_int() * 10 + new.unwrap_int())
    }))
}

fn check_insert(keys: &[Value], compare_native: Compare, compare_oracle: Compare) {
    let mut native = Rc::new(Map::Leaf);
    let mut oracle = native.clone();
    let mut previous = Vec::new();
    let mut random = 0x9e37_u64;
    for i in 0..800 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let k = keys[random as usize % keys.len()].clone();
        if i % 37 == 0 { previous.push((native.clone(), snapshot(&native))); }
        native = Data_Map_Internal_insertImpl(dummy_insert_oracle(), compare_native.clone(), k.clone(), Value::Int(i), native);
        oracle = Data_Map_Internal_insertPS(compare_oracle.clone(), k, Value::Int(i), oracle);
        assert_eq!(snapshot(&native), snapshot(&oracle), "native insert vs explicit PS oracle");
    }
    for (version, before) in previous { assert_eq!(snapshot(&version), before, "old insert version mutated"); }
}

fn check_insert_with(keys: &[Value], compare_native: Compare, compare_oracle: Compare) {
    let native_combines = Rc::new(AtomicUsize::new(0));
    let oracle_combines = Rc::new(AtomicUsize::new(0));
    let combine_native = counted_combine(native_combines.clone());
    let combine_oracle = counted_combine(oracle_combines.clone());
    let mut native = Rc::new(Map::Leaf);
    let mut oracle = native.clone();
    let mut previous = Vec::new();
    let mut random = 0x51ed_u64;
    for i in 0..800 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let k = keys[random as usize % keys.len()].clone();
        if i % 41 == 0 { previous.push((native.clone(), snapshot(&native))); }
        native = Data_Map_Internal_insertWithImpl(
            dummy_insert_with_oracle(), compare_native.clone(), combine_native.clone(), k.clone(), Value::Int(i), native);
        oracle = Data_Map_Internal_insertWithPS(compare_oracle.clone(), combine_oracle.clone(), k, Value::Int(i), oracle);
        assert_eq!(snapshot(&native), snapshot(&oracle), "native insertWith vs explicit PS oracle");
        assert_eq!(native_combines.load(AtomicOrdering::Relaxed), oracle_combines.load(AtomicOrdering::Relaxed), "combine callback counts");
    }
    for (version, before) in previous { assert_eq!(snapshot(&version), before, "old insertWith version mutated"); }
}

fn check_union_with(keys: &[Value], compare_native: Compare, compare_oracle: Compare) {
    let combine = Func2::Static(|old: Value, new: Value| Value::Int(old.unwrap_int() * 10 + new.unwrap_int()));
    for offset in 0..keys.len() {
        let mut a = Rc::new(Map::Leaf);
        let mut b = a.clone();
        for (i, k) in keys.iter().enumerate() {
            if i <= offset {
                a = Data_Map_Internal_insertPS(compare_oracle.clone(), k.clone(), Value::Int(i as i64), a);
            }
            if i >= offset {
                b = Data_Map_Internal_insertPS(compare_oracle.clone(), k.clone(), Value::Int(100 + i as i64), b);
            }
        }
        let before_a = snapshot(&a);
        let before_b = snapshot(&b);
        let expected = Data_Map_Internal_unionWithPS(compare_oracle.clone(), combine.clone(), a.clone(), b.clone());
        let native = Data_Map_Internal_unionWithImpl(dummy_union_oracle(), compare_native.clone(), combine.clone(), a.clone(), b.clone());
        assert_eq!(snapshot(&native), snapshot(&expected), "native unionWith vs explicit PS oracle");
        let biased = Data_Map_Internal_unionWithPS(compare_oracle.clone(), Func2::Static(|a: Value, _: Value| a), a.clone(), b.clone());
        let native_biased = Data_Map_Internal_unionWithImpl(
            dummy_union_oracle(), compare_native.clone(), Func2::Static(|a: Value, _: Value| a), a.clone(), b.clone());
        assert_eq!(snapshot(&native_biased), snapshot(&biased), "native left-biased union");
        assert_eq!(snapshot(&a), before_a, "left operand mutated");
        assert_eq!(snapshot(&b), before_b, "right operand mutated");
    }
}

fn collect_desc(map: &Map, out: &mut Vec<i64>) {
    match map {
        Map::Leaf => {}
        Map::Node(_, _, k, _, l, r) => {
            collect_desc(l, out);
            out.push(k.unwrap_int());
            collect_desc(r, out);
        }
    }
}

fn check_descending_order_and_callbacks() {
    let native_calls = Rc::new(AtomicUsize::new(0));
    let oracle_calls = Rc::new(AtomicUsize::new(0));
    let compare_native = counted_compare(native_calls.clone(), true);
    let compare_oracle = counted_compare(oracle_calls.clone(), true);
    let mut native = Rc::new(Map::Leaf);
    let mut oracle = native.clone();
    let mut previous = Vec::new();
    let keys: Vec<Value> = (0..64).map(Value::Int).collect();
    let mut random = 0x1234_u64;
    for i in 0..600 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let k = keys[random as usize % keys.len()].clone();
        if i % 41 == 0 { previous.push((native.clone(), snapshot(&native))); }
        native = Data_Map_Internal_insertImpl(dummy_insert_oracle(), compare_native.clone(), k.clone(), Value::Int(i), native);
        oracle = Data_Map_Internal_insertPS(compare_oracle.clone(), k, Value::Int(i), oracle);
        assert_eq!(snapshot(&native), snapshot(&oracle), "descending insert vs explicit PS oracle");
        assert_eq!(
            native_calls.load(AtomicOrdering::Relaxed),
            oracle_calls.load(AtomicOrdering::Relaxed),
            "descending comparator must be invoked exactly like the PS oracle");
    }
    let calls = native_calls.load(AtomicOrdering::Relaxed);
    assert!(calls > 600, "comparator must be exercised");
    assert!(calls <= 600 * 2 * 8, "comparator calls exceed the AVL path bound");
    let mut out = Vec::new();
    collect_desc(&native, &mut out);
    assert_eq!(out, (0..64).rev().collect::<Vec<i64>>(), "descending comparator must produce descending iteration");
    for (version, before) in previous { assert_eq!(snapshot(&version), before, "old descending version mutated"); }
}

fn main() {
    let ints: Vec<Value> = (-31..35).chain([i64::MIN, i64::MAX]).map(Value::Int).collect();
    let int_compare = {
        let ord = Purs_Data_Ord::Data_Ord_ordInt();
        ord.compare.clone()
    };
    check_insert(&ints, int_compare.clone(), int_compare.clone());
    check_insert_with(&ints, int_compare.clone(), int_compare.clone());
    check_union_with(&ints, int_compare.clone(), int_compare.clone());

    let string_compare = Purs_Data_Ord::Data_Ord_ordString().compare.clone();
    let strings: Vec<Value> = ["", "a", "A", "z", "é", "🦀", "😀", "\u{e000}", "a.b", "a_b"].iter()
        .map(|s| Value::String(purust_string_from_utf8(s))).chain([
            Value::String(purust_string_from_utf16(&[0xd800])), Value::String(purust_string_from_utf16(&[0xdc00]))
        ]).collect();
    check_insert(&strings, string_compare.clone(), string_compare.clone());
    check_insert_with(&strings, string_compare.clone(), string_compare.clone());
    check_union_with(&strings, string_compare.clone(), string_compare.clone());

    let qualified_compare = Purs_PureScript_Backend_Optimizer_CoreFn::PureScript_Backend_Optimizer_CoreFn_ordQualified(Purs_Data_Ord::Data_Ord_ordString()).compare.clone();
    let quals = qualified_keys();
    check_insert(&quals, qualified_compare.clone(), qualified_compare.clone());
    check_insert_with(&quals, qualified_compare.clone(), qualified_compare.clone());
    check_union_with(&quals, qualified_compare.clone(), qualified_compare.clone());

    check_descending_order_and_callbacks();

    println!("Native internal maps: explicit PS oracles vs native exports, exact AVL shape, Int/String/qualified keys (legacy + ClassShared when available), combine and comparator callback counts, descending bounds and persistence passed");
}
