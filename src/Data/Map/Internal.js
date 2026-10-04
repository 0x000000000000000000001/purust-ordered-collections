// FFI JavaScript de `Data.Map.Internal`.
//
// La lane Rust est native (Internal.rs) : pas de dictionnaire `Ord`, pas
// d'allocation `Value::Class` par comparaison ; le comparateur reste le
// callback PureScript, appelé via le `Func2` reçu, et ses opérandes traversent
// la frontière par copie `Value` (`a.clone()`) — seule la closure comparateur
// est empruntée, aucune comparaison n'est réimplémentée.
//
// Sur JS, l'oracle PureScript reste autoritaire : ces adaptateurs ne font
// qu'appliquer la fonction d'oracle reçue en premier argument, sans cycle
// d'import. Aucun adaptateur Go n'est fourni par ce paquet (la lane Go du
// pipeline utilise son propre paquet ordered-collections).
//
// Les implémentations d'oracle (`insertPS`, `insertWithPS`, `unionWithPS`)
// vivent dans le module PureScript et sont passées par le wrapper :
//
//   insert k v m = insertImpl insertPS compare k v m
//
// L'ordre des arguments est donc celui de la déclaration `foreign import`.
export const insertImpl = oracle => compare => k => v => m =>
    oracle(compare)(k)(v)(m);

export const insertWithImpl = oracle => compare => app => k => v => m =>
    oracle(compare)(app)(k)(v)(m);

export const unionWithImpl = oracle => compare => app => m1 => m2 =>
    oracle(compare)(app)(m1)(m2);
