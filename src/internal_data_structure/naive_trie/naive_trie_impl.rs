use crate::map::TrieLabel;
use super::naive_trie_b_f_iter::NaiveTrieBFIter;
use super::{NaiveTrie, NaiveTrieIntermOrLeaf, NaiveTrieRoot};
use std::vec::Drain;

impl<'trie, Label: Ord, Value> NaiveTrie<Label, Value> {
    pub fn make_root() -> Self {
        NaiveTrie::Root(NaiveTrieRoot { children: vec![] })
    }

    pub fn make_interm_or_leaf(label: TrieLabel<Label, Value>) -> Self {
        NaiveTrie::IntermOrLeaf(NaiveTrieIntermOrLeaf {
            children: vec![],
            label,
        })
    }

    pub fn make_interm(label: Label) -> Self {
        NaiveTrie::IntermOrLeaf(NaiveTrieIntermOrLeaf {
            children: vec![],
            label: TrieLabel::Label(label),
        })
    }

    pub fn make_leaf(value: Value) -> Self {
        NaiveTrie::IntermOrLeaf(NaiveTrieIntermOrLeaf {
            children: vec![],
            label: TrieLabel::Value(value),
        })
    }

    pub fn push<Arr: Iterator<Item = Label>>(&'trie mut self, mut word: Arr, value: Value) {
        let mut trie = self;
        // let mut word = word.map(TrieLabel::Label);
        while let Some(chr) = word.next() {
            let res = trie
                .children()
                .binary_search_by(|child| child.label().partial_cmp(&chr).unwrap());
            match res {
                Ok(j) => {
                    trie = match trie {
                        NaiveTrie::Root(node) => &mut node.children[j],
                        NaiveTrie::IntermOrLeaf(node) => &mut node.children[j],
                        _ => panic!("Unexpected type"),
                    };
                }
                Err(j) => {
                    let child_trie =
                        Self::make_interm(chr);
                    trie = match trie {
                        NaiveTrie::Root(node) => {
                            node.children.insert(j, child_trie);
                            &mut node.children[j]
                        }
                        NaiveTrie::IntermOrLeaf(node) => {
                            node.children.insert(j, child_trie);
                            &mut node.children[j]
                        }
                        _ => panic!("Unexpected type"),
                    };
                }
            };
        }
        match trie {
            NaiveTrie::Root(node) => Self::insert_or_set_value(&mut node.children, value),
            NaiveTrie::IntermOrLeaf(node) => Self::insert_or_set_value(&mut node.children, value),
            _ => panic!("Unexpected type"),
        }
    }

    fn insert_or_set_value(children: &mut Vec<NaiveTrie<Label, Value>>, value: Value) {
        match children.first_mut() {
            Some(ref mut x) => {
                match x {
                    NaiveTrie::Root(_) => (),
                    NaiveTrie::IntermOrLeaf(ref mut node) =>
                        if let TrieLabel::Value(ref mut v) = node.label {
                        *v = value;
                        return;
                    },
                    _ => panic!("Unexpected type"),
                }
            }
            _ => (),
        }
        children.insert(0, Self::make_leaf(value));
    }


    pub fn children(&self) -> &[Self] {
        match self {
            NaiveTrie::Root(node) => &node.children,
            NaiveTrie::IntermOrLeaf(node) => &node.children,
            _ => panic!("Unexpected type"),
        }
    }

    pub fn drain_children(&mut self) -> Drain<'_, Self> {
        match self {
            NaiveTrie::Root(node) => node.children.drain(0..),
            NaiveTrie::IntermOrLeaf(node) => node.children.drain(0..),
            _ => panic!("Unexpected type"),
        }
    }

    // /// # Panics
    // /// If self is not IntermOrLeaf.
    // #[allow(dead_code)]
    // pub fn value(&self) -> Option<&Value> {
    //     match self {
    //         NaiveTrie::IntermOrLeaf(node) => node.value.as_ref(),
    //         _ => panic!("Unexpected type"),
    //     }
    // }

    /// # Panics
    /// If self is not IntermOrLeaf.
    pub fn label(&self) -> &TrieLabel<Label, Value> {
        match self {
            NaiveTrie::IntermOrLeaf(node) => &node.label,
            _ => panic!("Unexpected type"),
        }
    }
}

impl<Label: Ord, Value> IntoIterator for NaiveTrie<Label, Value> {
    type Item = NaiveTrie<Label, Value>;
    type IntoIter = NaiveTrieBFIter<Label, Value>;
    fn into_iter(self) -> NaiveTrieBFIter<Label, Value> {
        NaiveTrieBFIter::new(self)
    }
}
