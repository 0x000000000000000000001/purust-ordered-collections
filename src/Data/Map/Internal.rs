use std::rc::Rc;

// Native AVL primitives for `Data.Map.Internal`.
//
// Purust appends this file to the generated `Purs_Data_Map_Internal` crate.
// Node layout, balancing and combine order are the ones of the PureScript
// implementation, so native and generated operations can be mixed freely and
// old versions stay valid (persistent path copying only).
//
// Each `*Impl` receives the PureScript oracle as its first argument and
// ignores it here: the JS FFI lane keeps that oracle authoritative. The Rust
// lane avoids the `Ord` dictionary and the per-comparison `Value::Class`/`Arc`
// wrappers, and rebuilds only the nodes on the insertion/union path. The
// comparator itself is the PureScript callback, called through the borrowed
// `Func2`; its operands cross the callback boundary by `Value` clone
// (`a.clone()`), so this is a borrowed comparator closure, not a borrowed-key
// comparator.
type KeyOrdering = std::cmp::Ordering;

fn purust_native_callback(
    compare: &Func2<Value, Value, Purs_Data_Ordering::Ordering>,
    a: &Value,
    b: &Value,
) -> KeyOrdering {
    match compare(a.clone(), b.clone()) {
        Purs_Data_Ordering::Ordering::LT => KeyOrdering::Less,
        Purs_Data_Ordering::Ordering::EQ => KeyOrdering::Equal,
        Purs_Data_Ordering::Ordering::GT => KeyOrdering::Greater,
    }
}

fn purust_native_height(map: &crate::Map) -> i64 {
    match map {
        crate::Map::Leaf => 0,
        crate::Map::Node(h, _, _, _, _, _) => *h,
    }
}

fn purust_native_size(map: &crate::Map) -> i64 {
    match map {
        crate::Map::Leaf => 0,
        crate::Map::Node(_, s, _, _, _, _) => *s,
    }
}

fn purust_native_node(
    key: Value,
    value: Value,
    left: Rc<crate::Map>,
    right: Rc<crate::Map>,
) -> Rc<crate::Map> {
    Rc::new(crate::Map::Node(
        1 + purust_native_height(&left).max(purust_native_height(&right)),
        1 + purust_native_size(&left) + purust_native_size(&right),
        key,
        value,
        left,
        right,
    ))
}

// Exact port of Data.Map.Internal.unsafeBalancedNode.
fn purust_native_balance(
    key: Value,
    value: Value,
    left: Rc<crate::Map>,
    right: Rc<crate::Map>,
) -> Rc<crate::Map> {
    let lh = purust_native_height(&left);
    let rh = purust_native_height(&right);
    if rh > lh + 1 {
        let crate::Map::Node(_, _, rk, rv, rl, rr) = right.as_ref() else { unreachable!() };
        if let crate::Map::Node(h, _, lk, lv, ll, lr) = rl.as_ref() {
            if *h > purust_native_height(rr) {
                return purust_native_node(
                    lk.clone(),
                    lv.clone(),
                    purust_native_node(key, value, left, ll.clone()),
                    purust_native_node(rk.clone(), rv.clone(), lr.clone(), rr.clone()),
                );
            }
        }
        return purust_native_node(
            rk.clone(),
            rv.clone(),
            purust_native_node(key, value, left, rl.clone()),
            rr.clone(),
        );
    }
    if lh > rh + 1 {
        let crate::Map::Node(_, _, lk, lv, ll, lr) = left.as_ref() else { unreachable!() };
        if let crate::Map::Node(h, _, rk, rv, rl, rr) = lr.as_ref() {
            if purust_native_height(ll) <= *h {
                return purust_native_node(
                    rk.clone(),
                    rv.clone(),
                    purust_native_node(lk.clone(), lv.clone(), ll.clone(), rl.clone()),
                    purust_native_node(key, value, rr.clone(), right),
                );
            }
        }
        return purust_native_node(
            lk.clone(),
            lv.clone(),
            ll.clone(),
            purust_native_node(key, value, lr.clone(), right),
        );
    }
    purust_native_node(key, value, left, right)
}

// Exact port of the tail-recursive `insert` in Data.Map.Internal.
fn purust_native_insert(
    compare: &impl Fn(&Value, &Value) -> KeyOrdering,
    key: Value,
    value: Value,
    map: &Rc<crate::Map>,
) -> Rc<crate::Map> {
    match map.as_ref() {
        crate::Map::Leaf => purust_native_node(key, value, map.clone(), map.clone()),
        crate::Map::Node(h, s, mk, mv, ml, mr) => match compare(&key, mk) {
            KeyOrdering::Less => purust_native_balance(
                mk.clone(),
                mv.clone(),
                purust_native_insert(compare, key, value, ml),
                mr.clone(),
            ),
            KeyOrdering::Greater => purust_native_balance(
                mk.clone(),
                mv.clone(),
                ml.clone(),
                purust_native_insert(compare, key, value, mr),
            ),
            KeyOrdering::Equal => Rc::new(crate::Map::Node(*h, *s, key, value, ml.clone(), mr.clone())),
        },
    }
}

// Exact port of the tail-recursive `insertWith` in Data.Map.Internal. The
// combining function receives the existing value first.
fn purust_native_insert_with(
    compare: &impl Fn(&Value, &Value) -> KeyOrdering,
    combine: &impl Fn(Value, Value) -> Value,
    key: Value,
    value: Value,
    map: &Rc<crate::Map>,
) -> Rc<crate::Map> {
    match map.as_ref() {
        crate::Map::Leaf => purust_native_node(key, value, map.clone(), map.clone()),
        crate::Map::Node(h, s, mk, mv, ml, mr) => match compare(&key, mk) {
            KeyOrdering::Less => purust_native_balance(
                mk.clone(),
                mv.clone(),
                purust_native_insert_with(compare, combine, key, value, ml),
                mr.clone(),
            ),
            KeyOrdering::Greater => purust_native_balance(
                mk.clone(),
                mv.clone(),
                ml.clone(),
                purust_native_insert_with(compare, combine, key, value, mr),
            ),
            KeyOrdering::Equal => Rc::new(crate::Map::Node(
                *h,
                *s,
                key,
                combine(mv.clone(), value),
                ml.clone(),
                mr.clone(),
            )),
        },
    }
}

// Exact port of Data.Map.Internal.unsafeSplit.
fn purust_native_split(
    compare: &impl Fn(&Value, &Value) -> KeyOrdering,
    key: &Value,
    map: &Rc<crate::Map>,
) -> (Option<Value>, Rc<crate::Map>, Rc<crate::Map>) {
    match map.as_ref() {
        crate::Map::Leaf => (None, map.clone(), map.clone()),
        crate::Map::Node(_, _, mk, mv, ml, mr) => match compare(key, mk) {
            KeyOrdering::Less => {
                let (value, ll, lr) = purust_native_split(compare, key, ml);
                (value, ll, purust_native_balance(mk.clone(), mv.clone(), lr, mr.clone()))
            }
            KeyOrdering::Greater => {
                let (value, rl, rr) = purust_native_split(compare, key, mr);
                (value, purust_native_balance(mk.clone(), mv.clone(), ml.clone(), rl), rr)
            }
            KeyOrdering::Equal => (Some(mv.clone()), ml.clone(), mr.clone()),
        },
    }
}

// Exact port of Data.Map.Internal.unsafeUnionWith.
fn purust_native_union(
    compare: &impl Fn(&Value, &Value) -> KeyOrdering,
    combine: &impl Fn(Value, Value) -> Value,
    left: &Rc<crate::Map>,
    right: &Rc<crate::Map>,
) -> Rc<crate::Map> {
    match (left.as_ref(), right.as_ref()) {
        (crate::Map::Leaf, _) => right.clone(),
        (_, crate::Map::Leaf) => left.clone(),
        (_, crate::Map::Node(_, _, rk, rv, rl, rr)) => {
            let (lv, ll, lr) = purust_native_split(compare, rk, left);
            let l = purust_native_union(compare, combine, &ll, rl);
            let r = purust_native_union(compare, combine, &lr, rr);
            let value = match lv {
                Some(lv) => combine(lv, rv.clone()),
                None => rv.clone(),
            };
            purust_native_balance(rk.clone(), value, l, r)
        }
    }
}

pub fn Data_Map_Internal_insertImpl(
    _oracle: Func4<Func2<Value, Value, Purs_Data_Ordering::Ordering>, Value, Value, Rc<crate::Map>, Rc<crate::Map>>,
    compare: Func2<Value, Value, Purs_Data_Ordering::Ordering>,
    key: Value,
    value: Value,
    map: Rc<crate::Map>,
) -> Rc<crate::Map> {
    purust_native_insert(&|a, b| purust_native_callback(&compare, a, b), key, value, &map)
}

pub fn Data_Map_Internal_insertWithImpl(
    _oracle: Func5<Func2<Value, Value, Purs_Data_Ordering::Ordering>, Func2<Value, Value, Value>, Value, Value, Rc<crate::Map>, Rc<crate::Map>>,
    compare: Func2<Value, Value, Purs_Data_Ordering::Ordering>,
    combine: Func2<Value, Value, Value>,
    key: Value,
    value: Value,
    map: Rc<crate::Map>,
) -> Rc<crate::Map> {
    purust_native_insert_with(
        &|a, b| purust_native_callback(&compare, a, b),
        &|old, new| combine(old, new),
        key,
        value,
        &map,
    )
}

pub fn Data_Map_Internal_unionWithImpl(
    _oracle: Func4<Func2<Value, Value, Purs_Data_Ordering::Ordering>, Func2<Value, Value, Value>, Rc<crate::Map>, Rc<crate::Map>, Rc<crate::Map>>,
    compare: Func2<Value, Value, Purs_Data_Ordering::Ordering>,
    combine: Func2<Value, Value, Value>,
    left: Rc<crate::Map>,
    right: Rc<crate::Map>,
) -> Rc<crate::Map> {
    purust_native_union(
        &|a, b| purust_native_callback(&compare, a, b),
        &|a, b| combine(a, b),
        &left,
        &right,
    )
}
