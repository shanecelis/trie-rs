use super::NaiveTrie;
use std::collections::VecDeque;

#[derive(Debug)]
/// Iterates over NaiveTrie in Breadth-First manner.
pub struct NaiveTrieBFIter<Label, Value> {
    unvisited: VecDeque<NaiveTrie<Label, Value>>,
}

impl<Label, Value> NaiveTrieBFIter<Label, Value> {
    pub fn new(iter_start: NaiveTrie<Label, Value>) -> Self {
        let mut unvisited = VecDeque::new();
        unvisited.push_back(iter_start);
        Self { unvisited }
    }
}

impl<Label: Ord, Value> Iterator for NaiveTrieBFIter<Label, Value> {
    type Item = NaiveTrie<Label, Value>;

    /// Returns:
    ///
    /// - None: All nodes are visited.
    /// - Some(NaiveTrie::Root): Root node.
    /// - Some(NaiveTrie::IntermOrLeaf): Intermediate or leaf node.
    /// - Some(NaiveTrie::PhantomSibling): Marker to represent "all siblings are iterated".
    fn next(&mut self) -> Option<Self::Item> {
        self.unvisited.pop_front().map(|mut trie| {
            match trie {
                NaiveTrie::Root(_) | NaiveTrie::IntermOrLeaf(_) => {
                    for child in trie.drain_children() {
                        self.unvisited.push_back(child);
                    }
                    self.unvisited.push_back(NaiveTrie::PhantomSibling);
                }
                NaiveTrie::PhantomSibling => {}
            };
            trie
        })
    }
}

#[cfg(test)]
mod bf_iter_tests {
    type NaiveTrie<T> = super::NaiveTrie<T, ()>;
    const TERMINAL: () = ();
    // const FALSE: Option<()> = None;

    macro_rules! parameterized_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (words, expected_nodes) = $value;
                let mut trie = NaiveTrie::make_root();
                for word in words {
                    trie.push(word.bytes().into_iter(), ());
                }
                let nodes: Vec<NaiveTrie<u8>> = trie.clone().into_iter().collect();
                assert_eq!(nodes.len(), expected_nodes.len(), "different lengths");
                for i in 0..nodes.len() {
                    let node = &nodes[i];
                    let expected_node = &expected_nodes[i];

                    assert!(std::mem::discriminant(node) == std::mem::discriminant(expected_node), "discriminant failure on node {i}");

                    if let NaiveTrie::IntermOrLeaf(n) = node {
                        assert_eq!(n.label, *expected_node.label(), "different labels on {i}");
                        // assert_eq!(n.label as char, *expected_node.label() as char);
                        // assert_eq!(n.value.is_some(), expected_node.value().is_some());
                    }
                }
                let t = crate::Trie(crate::map::TrieBuilder { naive_trie: trie }.build());
                eprintln!("{:?}", t.postfix_search("").collect::<Vec<String>>());

            }
        )*
        }
    }

    parameterized_tests! {
        t1: (
            Vec::<&str>::new(),
            vec![
                NaiveTrie::make_root(),
                // parent = root
                NaiveTrie::PhantomSibling,
            ]
        ),
        t2: (
            vec!["a"],
            vec![
                NaiveTrie::make_root(),
                // parent = root
                NaiveTrie::make_interm(b'a'),
                NaiveTrie::PhantomSibling,
                // parent = a
                NaiveTrie::make_leaf(TERMINAL),
                NaiveTrie::PhantomSibling,
                // parent = a terminal
                NaiveTrie::PhantomSibling,
            ]
        ),
        t3: (
            vec!["a", "a"],
            vec![
                NaiveTrie::make_root(),
                // parent = root
                NaiveTrie::make_interm(b'a'),
                NaiveTrie::PhantomSibling,
                // parent = a
                NaiveTrie::make_leaf(TERMINAL),
                NaiveTrie::PhantomSibling,
                // parent = a terminal
                NaiveTrie::PhantomSibling,
            ]
        ),
        t3b: (
            vec!["a", "b"],
            vec![
                NaiveTrie::make_root(),
                // parent = root
                NaiveTrie::make_interm(b'a'),
                NaiveTrie::make_interm(b'b'),
                NaiveTrie::PhantomSibling,
                // parent = a
                NaiveTrie::make_leaf(TERMINAL),
                NaiveTrie::PhantomSibling,
                // parent = b
                NaiveTrie::make_leaf(TERMINAL),
                NaiveTrie::PhantomSibling,
                // parent = a terminal
                NaiveTrie::PhantomSibling,
                // parent = b terminal
                NaiveTrie::PhantomSibling,
            ]
        ),
        t4b: (
            // root
            //  |-----------------------+-----------------------+
            //  |                       |                       |
            //  a (term)                b                       Ph
            //  |---------+             |-----------------+
            //  |         |             |                 |
            //  n (term)  Ph            a                 Ph
            //  |                       |--------+
            //  |                       |        |
            //  Ph                      d (term) Ph
            //                          |
            //                          |
            //                          Ph
            vec!["a", "bad", "an"],
            vec![
                NaiveTrie::make_root(),
                // parent = root
                NaiveTrie::make_interm(b'a'),
                NaiveTrie::make_interm(b'b'),
                NaiveTrie::PhantomSibling,
                // parent = [a]
                NaiveTrie::make_leaf(TERMINAL),
                NaiveTrie::make_interm(b'n'), // i = 5
                NaiveTrie::PhantomSibling,
                // parent = [a] terminal
                // parent = b
                NaiveTrie::make_interm(b'a'),
                NaiveTrie::PhantomSibling,
                // parent = n
                NaiveTrie::PhantomSibling,
                NaiveTrie::make_leaf(TERMINAL),// i = 10
                NaiveTrie::PhantomSibling,
                // parent = b[a]d
                NaiveTrie::make_interm(b'd'),
                NaiveTrie::PhantomSibling,
                // parent = d
                NaiveTrie::PhantomSibling,
                NaiveTrie::make_leaf(TERMINAL),// i = 15
                // parent = d terminal
                NaiveTrie::PhantomSibling,
                // parent = n terminal
                NaiveTrie::PhantomSibling,
            ]
        ),
        // t5: (
        //     // 'り' => 227, 130, 138
        //     // 'ん' => 227, 130, 147
        //     // 'ご' => 227, 129, 148
        //     vec!["a", "an", "りんご", "りんりん"],
        //     vec![
        //         NaiveTrie::make_root(),
        //         // parent = root
        //         NaiveTrie::make_interm(b'a'),
        //         NaiveTrie::make_interm(227),
        //         NaiveTrie::PhantomSibling,
        //         // parent = a
        //         NaiveTrie::make_leaf(TERMINAL),
        //         NaiveTrie::make_interm(b'n'),
        //         NaiveTrie::PhantomSibling,
        //         // parent = [227] 130 138 (り)
        //         NaiveTrie::make_interm(130),
        //         NaiveTrie::PhantomSibling,
        //         // parent = n
        //         NaiveTrie::make_leaf(TERMINAL),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 [130] 138 (り)
        //         NaiveTrie::make_interm(138),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 130 [138] (り)
        //         NaiveTrie::make_interm(227),
        //         NaiveTrie::PhantomSibling,
        //         // parent = [227] 130 147 (ん)
        //         NaiveTrie::make_interm(130),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 [130] 147 (ん)
        //         NaiveTrie::make_interm(147),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 130 [147] (ん)
        //         NaiveTrie::make_interm(227),
        //         NaiveTrie::PhantomSibling,
        //         // parent = [227] _ _ (ご or り)
        //         NaiveTrie::make_interm(129),
        //         NaiveTrie::make_interm(130),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 [129] 148 (ご)
        //         NaiveTrie::make_interm(148),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 [130] 138 (り)
        //         NaiveTrie::make_interm(138),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 129 [148] (ご)
        //         NaiveTrie::make_leaf(TERMINAL),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 130 [138] (り)
        //         NaiveTrie::make_interm(227),
        //         NaiveTrie::PhantomSibling,
        //         // parent = [227] 130 147 (ん)
        //         NaiveTrie::make_interm(130),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 [130] 147 (ん)
        //         NaiveTrie::make_interm(147),
        //         NaiveTrie::PhantomSibling,
        //         // parent = 227 130 [147] (ん)
        //         NaiveTrie::make_leaf(TERMINAL),
        //         NaiveTrie::PhantomSibling,
        //     ]
        // ),
    }
}
