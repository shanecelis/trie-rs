#[derive(Debug, Clone)]
/// Strips off `Value`s from [crate::map::Trie].
pub struct Keys<I>(I);

impl<I> Keys<I> {
    ///
    pub fn new(iter: I) -> Self {
        Self(iter)
    }
}

// TODO: This is generic for V, which is a stand-in for the Value, but in a
// `map::Trie<K,V>`, its iterators will actually reurn `(C, &V)`. Hopefully that
// won't matter.
impl<I, C, V> Iterator for Keys<I>
where
    I: Iterator<Item = (C, V)>,
{
    type Item = C;
    fn next(&mut self) -> Option<C> {
        self.0.next().map(|x| x.0)
    }
}

/// Strip an iterator items `(K, V)` to only have `K`.
pub trait KeysExt: Iterator {
    /// Use [entries] to strip values from a [crate::iter] iterator.
    fn keys(self) -> Keys<Self>
    where
        Self: Sized,
    {
        Keys::new(self)
    }
}

impl<T> KeysExt for T where T: Iterator + ?Sized {}
