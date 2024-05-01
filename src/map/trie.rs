//! A trie map stores a value with each word or key.
use super::{Trie, TrieLabel};
use crate::inc_search::IncSearch;
use crate::iter::{PostfixIter, PrefixIter, SearchIter};
use crate::try_collect::{TryCollect, TryFromIterator};
use louds_rs::{ChildNodeIter, LoudsNodeNum, AncestorNodeIter};
use std::{iter::FromIterator, cmp::{PartialOrd, Ordering}};

impl<Label: Ord, Value> Trie<Label, Value> {
    /// Return `Some(&Value)` if query is an exact match.
    pub fn exact_match(&self, query: impl AsRef<[Label]>) -> Option<&Value> {
        self.exact_match_node(query)
            .and_then(move |x| self.value(x))
    }

    /// Return `Node` if query is an exact match.
    #[inline]
    fn exact_match_node(&self, query: impl AsRef<[Label]>) -> Option<LoudsNodeNum> {
        let mut cur_node_num = LoudsNodeNum(1);

        for (i, chr) in query.as_ref().iter().enumerate() {
            let children_node_nums: Vec<LoudsNodeNum> =
                self.children_node_nums(cur_node_num).collect();
            let res = self.bin_search_by_children_labels(chr, &children_node_nums[..]);

            match res {
                Ok(j) => {
                    let child_node_num = children_node_nums[j];
                    if i == query.as_ref().len() - 1 && self.is_terminal(child_node_num) {
                        return Some(child_node_num);
                    }
                    cur_node_num = child_node_num;
                }
                Err(_) => return None,
            }
        }
        None
    }

    /// Return `Some(&mut value)` if query is an exact match.
    pub fn exact_match_mut(&mut self, query: impl AsRef<[Label]>) -> Option<&mut Value> {
        self.exact_match_node(query)
            .and_then(move |x| self.value_mut(x))
    }

    /// Create an incremental search. Useful for interactive applications. See
    /// [crate::inc_search] for details.
    pub fn inc_search(&self) -> IncSearch<'_, Label, Value> {
        IncSearch::new(self)
    }

    /// Return true if `query` is a prefix.
    ///
    /// Note: A prefix may be an exact match or not, and an exact match may be a
    /// prefix or not.
    pub fn is_prefix(&self, query: impl AsRef<[Label]>) -> bool {
        let mut cur_node_num = LoudsNodeNum(1);

        for chr in query.as_ref().iter() {
            let children_node_nums: Vec<_> = self.children_node_nums(cur_node_num).collect();
            let res = self.bin_search_by_children_labels(chr, &children_node_nums[..]);
            match res {
                Ok(j) => cur_node_num = children_node_nums[j],
                Err(_) => return false,
            }
        }
        // Are there more nodes after our query?
        self.has_children_node_nums(cur_node_num)
    }

    /// Return all entries and their values that match `query`.
    pub fn predictive_search<C, M>(
        &self,
        query: impl AsRef<[Label]>,
    ) -> SearchIter<'_, Label, Value, C, M>
    where
        C: TryFromIterator<Label, M> + Clone,
        Label: Clone,
    {
        SearchIter::new(self, query)
    }

    /// Return the postfixes and values of all entries that match `query`.
    pub fn postfix_search<C, M>(
        &self,
        query: impl AsRef<[Label]>,
    ) -> PostfixIter<'_, Label, Value, C, M>
    where
        C: TryFromIterator<Label, M>,
        Label: Clone,
    {
        let mut cur_node_num = LoudsNodeNum(1);

        // Consumes query (prefix)
        for chr in query.as_ref() {
            let children_node_nums: Vec<_> = self.children_node_nums(cur_node_num).collect();
            let res = self.bin_search_by_children_labels(chr, &children_node_nums[..]);
            match res {
                Ok(i) => cur_node_num = children_node_nums[i],
                Err(_) => {
                    return PostfixIter::empty(self);
                }
            }
        }

        PostfixIter::new(self, cur_node_num)
    }

    /// Return the common prefixes of `query`.
    pub fn common_prefix_search<C, M>(
        &self,
        query: impl AsRef<[Label]>,
    ) -> PrefixIter<'_, Label, Value, C, M>
    where
        C: TryFromIterator<Label, M>,
        Label: Clone,
    {
        PrefixIter::new(self, query)
    }

    /// Return the longest shared prefix or terminal of `query`.
    pub fn longest_prefix<C, M>(&self, query: impl AsRef<[Label]>) -> Option<C>
    where
        C: TryFromIterator<Label, M>,
        Label: Clone,
    {
        let mut cur_node_num = LoudsNodeNum(1);
        let mut buffer = Vec::new();

        // Consumes query (prefix)
        for chr in query.as_ref() {
            let children_node_nums: Vec<_> = self.children_node_nums(cur_node_num).collect();
            let res = self.bin_search_by_children_labels(chr, &children_node_nums[..]);
            match res {
                Ok(i) => {
                    cur_node_num = children_node_nums[i];
                    buffer.push(cur_node_num);
                }
                Err(_) => {
                    return None;
                }
            }
        }

        // Walk the trie as long as there is only one path and it isn't a terminal value.
        while !self.is_terminal(cur_node_num) {
            let mut iter = self.children_node_nums(cur_node_num);
            let first = iter.next();
            let second = iter.next();
            match (first, second) {
                (Some(child_node_num), None) => {
                    cur_node_num = child_node_num;
                    buffer.push(child_node_num);
                }
                _ => break,
            }
        }
        if buffer.is_empty() {
            None
        } else {
            Some(
                buffer
                    .into_iter()
                    .map(|x| self.label(x).clone())
                    .try_collect()
                    .expect("Could not collect"),
            )
        }
    }

    pub(crate) fn has_children_node_nums(&self, node_num: LoudsNodeNum) -> bool {
        self.louds
            .parent_to_children_indices(node_num)
            .next()
            .is_some()
    }

    pub(crate) fn children_node_nums(&self, node_num: LoudsNodeNum) -> ChildNodeIter {
        self.louds.parent_to_children_nodes(node_num)
    }

    pub(crate) fn bin_search_by_children_labels(
        &self,
        query: &Label,
        children_node_nums: &[LoudsNodeNum],
    ) -> Result<usize, usize> {
        // children_node_nums.binary_search_by(|child_node_num| self.trie_label(*child_node_num).cmp(query))
        children_node_nums.binary_search_by(|child_node_num| self.label(*child_node_num).cmp(query))
    }

    pub(crate) fn trie_label(&self, node_num: LoudsNodeNum) -> &TrieLabel<Label, Value> {
        &self.trie_labels[(node_num.0 - 2) as usize]
    }

    pub(crate) fn trie_label_mut(&mut self, node_num: LoudsNodeNum) -> &mut TrieLabel<Label, Value> {
        &mut self.trie_labels[(node_num.0 - 2) as usize]
    }

    pub(crate) fn label(&self, node_num: LoudsNodeNum) -> &Label {
        match &self.trie_labels[(node_num.0 - 2) as usize] {
            TrieLabel::Label(l) => l,
            TrieLabel::Value(_) => panic!("label() called on value"),
        }
    }

    pub(crate) fn is_terminal(&self, node_num: LoudsNodeNum) -> bool {
        if node_num.0 >= 2 {
            self.children_node_nums(node_num)
                .next().map(|x| matches!(self.trie_label(x), TrieLabel::Value(_)))
                .unwrap_or(false)
        } else {
            false
        }
    }

    pub(crate) fn is_prefix_node(&self, node_num: LoudsNodeNum) -> bool {
        if node_num.0 >= 2 {
            self.children_node_nums(node_num)
                .filter(|x| ! matches!(self.trie_label(*x), TrieLabel::Value(_)))
                .next()
                .is_some()
        } else {
            true
        }
    }

    pub(crate) fn value(&self, node_num: LoudsNodeNum) -> Option<&Value> {
        if node_num.0 >= 2 {
            self.children_node_nums(node_num)
                .next().and_then(|x| match self.trie_label(x) {
                    TrieLabel::Value(ref x) => Some(x),
                    _ => None,
                })
        } else {
            None
        }
    }

    pub(crate) fn value_mut(&mut self, node_num: LoudsNodeNum) -> Option<&mut Value> {
        if node_num.0 >= 2 {
            self.children_node_nums(node_num)
                .next().and_then(|x| match self.trie_label_mut(x) {
                    TrieLabel::Value(ref mut x) => Some(x),
                    _ => None,
                })
        } else {
            None
        }
    }

    // pub(crate) fn value_mut(&mut self, node_num: LoudsNodeNum) -> Option<&mut Value> {
    //     self.trie_labels[(node_num.0 - 2) as usize].value.as_mut()
    // }

    pub (crate) fn child_to_ancestors(&self, node_num: LoudsNodeNum) -> AncestorNodeIter {
        self.louds.child_to_ancestors(node_num)
    }
}

impl<Label, Value, C> FromIterator<(C, Value)> for Trie<Label, Value>
where
    C: AsRef<[Label]>,
    Label: Ord + Clone,
{
    fn from_iter<T>(iter: T) -> Self
    where
        Self: Sized,
        T: IntoIterator<Item = (C, Value)>,
    {
        let mut builder = super::TrieBuilder::new();
        for (k, v) in iter {
            builder.push(k, v)
        }
        builder.build()
    }
}

impl<Label: PartialOrd, Value: PartialEq> PartialOrd for TrieLabel<Label, Value> {
    #[inline]
    fn partial_cmp(
        &self,
        other: &TrieLabel<Label, Value>,
    ) -> Option<Ordering> {
        match (self, other) {
            (TrieLabel::Label(a), TrieLabel::Label(b)) => {
                PartialOrd::partial_cmp(a, b)
            }
            (TrieLabel::Value(_), TrieLabel::Value(_)) => {
                panic!("There should never be more than one value in a set of leaves.")
            }
            (TrieLabel::Label(_), TrieLabel::Value(_)) => {
                Some(Ordering::Greater)
            }
            (TrieLabel::Value(_), TrieLabel::Label(_)) => {
                Some(Ordering::Less)
            }
        }
    }
}

impl<Label: PartialOrd, Value> PartialOrd<Label> for TrieLabel<Label, Value> {
    // #[inline]
    fn partial_cmp(
        &self,
        other: &Label,
    ) -> Option<Ordering> {
        match self {
            TrieLabel::Label(a) => {
                PartialOrd::partial_cmp(a, other)
            }
            TrieLabel::Value(_) => {
                Some(Ordering::Less)
            }
        }
    }
}

impl<Label: PartialOrd, Value: PartialEq> Ord for TrieLabel<Label, Value> {
    // Required method
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<Label: PartialEq, Value> PartialEq<Label> for TrieLabel<Label, Value> {
    #[inline]
    fn eq(&self, other: &Label) -> bool {
        match self {
            TrieLabel::Label(a) => {
                PartialEq::eq(a, other)
            }
            TrieLabel::Value(_) => {
                false
            }
        }
    }
}

// impl<Label: PartialEq, Value: PartialEq> PartialEq for TrieLabel<Label, Value> {
//     #[inline]
//     fn eq(&self, other: &Self) -> bool {
//         match (self, other) {
//             (TrieLabel::Label(a), TrieLabel::Label(b)) => {
//                 PartialEq::partial_eq(a, b)
//             }
//             (TrieLabel::Value(a), TrieLabel::Value(b)) => {
//                 PartialEq::partial_eq(a, b)
//                 // panic!("There should never be more than one value in a set of leaves.")
//             }
//             (TrieLabel::Label(a), TrieLabel::Value(b)) => {
//                 false
//             }
//             (TrieLabel::Value(a), TrieLabel::Label(b)) => {
//                 false
//             }
//         }
//     }
// }

impl<Label: PartialEq, Value: PartialEq> PartialEq for TrieLabel<Label, Value> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (TrieLabel::Label(a), TrieLabel::Label(b)) => {
                PartialEq::eq(a, b)
            }
            (TrieLabel::Value(a), TrieLabel::Value(b)) => {
                // false
                PartialEq::eq(a, b)
                // panic!("There should never be more than one value in a set of leaves.")
            }
            (TrieLabel::Label(_), TrieLabel::Value(_)) => {
                false
            }
            (TrieLabel::Value(_), TrieLabel::Label(_)) => {
                false
            }
        }
    }
}

impl<Label: PartialEq, Value: PartialEq> Eq for TrieLabel<Label, Value> { }



#[cfg(test)]
mod search_tests {
    use crate::map::{Trie, TrieBuilder};
    use std::iter::FromIterator;

    fn build_trie() -> Trie<u8, u8> {
        let mut builder = TrieBuilder::new();
        builder.push("a", 0);
        builder.push("app", 1);
        builder.push("apple", 2);
        builder.push("better", 3);
        builder.push("application", 4);
        builder.push("アップル🍎", 5);
        builder.build()
    }

    fn build_trie2() -> Trie<char, u8> {
        let mut builder: TrieBuilder<char, u8> = TrieBuilder::new();
        builder.insert("a".chars(), 0);
        builder.insert("app".chars(), 1);
        builder.insert("apple".chars(), 2);
        builder.insert("better".chars(), 3);
        builder.insert("application".chars(), 4);
        builder.insert("アップル🍎".chars(), 5);
        builder.build()
    }

    #[test]
    fn sanity_check() {
        let trie = build_trie();
        let v: Vec<(String, &u8)> = trie.predictive_search("apple").collect();
        assert_eq!(v, vec![("apple".to_string(), &2)]);
    }

    #[test]
    fn clone() {
        let trie = build_trie();
        let _c: Trie<u8, u8> = trie.clone();
    }

    #[test]
    fn value_mut() {
        let mut trie = build_trie();
        assert_eq!(trie.exact_match("apple"), Some(&2));
        let v = trie.exact_match_mut("apple").unwrap();
        *v = 10;
        assert_eq!(trie.exact_match("apple"), Some(&10));
    }

    #[test]
    fn trie_from_iter() {
        let trie = Trie::<u8, u8>::from_iter([
            ("a", 0),
            ("app", 1),
            ("apple", 2),
            ("better", 3),
            ("application", 4),
        ]);
        assert_eq!(trie.exact_match("application"), Some(&4));
    }

    #[test]
    fn collect_a_trie() {
        // Does not work with arrays in rust 2018 because into_iter() returns references instead of owned types.
        // let trie: Trie<u8, u8> = [("a", 0), ("app", 1), ("apple", 2), ("better", 3), ("application", 4)].into_iter().collect();
        let trie: Trie<u8, u8> = vec![
            ("a", 0),
            ("app", 1),
            ("apple", 2),
            ("better", 3),
            ("application", 4),
        ]
        .into_iter()
        .collect();
        assert_eq!(trie.exact_match("application"), Some(&4));
    }

    #[test]
    fn use_empty_queries() {
        let trie = build_trie();
        assert!(trie.exact_match("").is_none());
        let _ = trie.predictive_search::<String, _>("").next();
        let _ = trie.postfix_search::<String, _>("").next();
        let _ = trie.common_prefix_search::<String, _>("").next();
    }

    mod exact_match_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_match) = $value;
                    let trie = super::build_trie();
                    let result = trie.exact_match(query);
                    assert_eq!(result, expected_match);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", Some(&0)),
            t2: ("app", Some(&1)),
            t3: ("apple", Some(&2)),
            t4: ("application", Some(&4)),
            t5: ("better", Some(&3)),
            t6: ("アップル🍎", Some(&5)),
            t7: ("appl", None),
            t8: ("appler", None),
        }
    }

    mod is_prefix_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_match) = $value;
                    let trie = super::build_trie();
                    let result = trie.is_prefix(query);
                    assert_eq!(result, expected_match);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", true),
            t2: ("app", true),
            t3: ("apple", false),
            t4: ("application", false),
            t5: ("better", false),
            t6: ("アップル🍎", false),
            t7: ("appl", true),
            t8: ("appler", false),
            t9: ("アップル", true),
        }
    }

    mod longest_prefix_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_match) = $value;
                    let trie = super::build_trie();
                    let result: Option<String> = trie.longest_prefix(query);
                    let expected_match = expected_match.map(str::to_string);
                    assert_eq!(result, expected_match);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", Some("a")),
            t2: ("ap", Some("app")),
            t3: ("appl", Some("appl")),
            t4: ("appli", Some("application")),
            t5: ("b", Some("better")),
            t6: ("アップル🍎", Some("アップル🍎")),
            t7: ("appler", None),
            t8: ("アップル", Some("アップル🍎")),
            t9: ("z", None),
            t10: ("applesDONTEXIST", None),
            t11: ("", None),
        }
    }

    mod predictive_search_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_results) = $value;
                    let trie = super::build_trie();
                    let results: Vec<(String, &u8)> = trie.predictive_search(query).collect();
                    let expected_results: Vec<(String, &u8)> = expected_results.iter().map(|s| (s.0.to_string(), &s.1)).collect();
                    assert_eq!(results, expected_results);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", vec![("a", 0), ("app", 1), ("apple", 2), ("application", 4)]),
            t2: ("app", vec![("app", 1), ("apple", 2), ("application", 4)]),
            t3: ("appl", vec![("apple", 2), ("application", 4)]),
            t4: ("apple", vec![("apple", 2)]),
            t5: ("b", vec![("better", 3)]),
            t6: ("c", Vec::<(&str, u8)>::new()),
            t7: ("アップ", vec![("アップル🍎", 5)]),
        }
    }

    mod common_prefix_search_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_results) = $value;
                    let trie = super::build_trie();
                    let results: Vec<(String, &u8)> = trie.common_prefix_search(query).collect();
                    let expected_results: Vec<(String, &u8)> = expected_results.iter().map(|s| (s.0.to_string(), &s.1)).collect();
                    assert_eq!(results, expected_results);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", vec![("a", 0)]),
            t2: ("ap", vec![("a", 0)]),
            t3: ("appl", vec![("a", 0), ("app", 1)]),
            t4: ("appler", vec![("a", 0), ("app", 1), ("apple", 2)]),
            t5: ("bette", Vec::<(&str, u8)>::new()),
            t6: ("betterment", vec![("better", 3)]),
            t7: ("c", Vec::<(&str, u8)>::new()),
            t8: ("アップル🍎🍏", vec![("アップル🍎", 5)]),
        }
    }

    mod postfix_search_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_results) = $value;
                    let trie = super::build_trie();
                    let results: Vec<(String, &u8)> = trie.postfix_search(query).collect();
                    let expected_results: Vec<(String, &u8)> = expected_results.iter().map(|s| (s.0.to_string(), &s.1)).collect();
                    assert_eq!(results, expected_results);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", vec![("pp", 1), ("pple", 2), ("pplication", 4)]),
            t2: ("ap", vec![("p", 1), ("ple", 2), ("plication", 4)]),
            t3: ("appl", vec![("e", 2), ("ication", 4)]),
            t4: ("appler", Vec::<(&str, u8)>::new()),
            t5: ("bette", vec![("r", 3)]),
            t6: ("betterment", Vec::<(&str, u8)>::new()),
            t7: ("c", Vec::<(&str, u8)>::new()),
            t8: ("アップル🍎🍏", Vec::<(&str, u8)>::new()),
        }
    }

    mod postfix_search_char_tests {
        macro_rules! parameterized_tests {
            ($($name:ident: $value:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (query, expected_results) = $value;
                    let trie = super::build_trie2();
                    let chars: Vec<char> = query.chars().collect();
                    let results: Vec<(String, &u8)> = trie.postfix_search(chars).collect();
                    let expected_results: Vec<(String, &u8)> = expected_results.iter().map(|s| (s.0.to_string(), &s.1)).collect();
                    assert_eq!(results, expected_results);
                }
            )*
            }
        }

        parameterized_tests! {
            t1: ("a", vec![("pp", 1), ("pple", 2), ("pplication", 4)]),
            t2: ("ap", vec![("p", 1), ("ple", 2), ("plication", 4)]),
            t3: ("appl", vec![("e", 2), ("ication", 4)]),
            t4: ("appler", Vec::<(&str, u8)>::new()),
            t5: ("bette", vec![("r", 3)]),
            t6: ("betterment", Vec::<(&str, u8)>::new()),
            t7: ("c", Vec::<(&str, u8)>::new()),
            t8: ("アップル🍎🍏", Vec::<(&str, u8)>::new()),
        }
    }
}
