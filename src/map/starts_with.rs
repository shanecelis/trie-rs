use crate::map::{Trie, node_ref::NodeRef};
use crate::try_from::TryFromTokens;
use louds_rs::LoudsNodeNum;

/// Iterate over the match/terminal node refs.
pub struct StartsWith<'t, Token, Value> {
    pub(crate) trie: &'t Trie<Token, Value>,
    pub(crate) queue: Vec<LoudsNodeNum>,
    pub(crate) start: LoudsNodeNum,
}

impl<'t, Token, Value> StartsWith<'t, Token, Value> {
    #[inline]
    pub(crate) fn new(trie: &'t Trie<Token, Value>, start: LoudsNodeNum) -> Self {
        Self {
            trie,
            queue: vec![start],
            start,
        }
    }

    #[inline]
    pub(crate) fn empty(trie: &'t Trie<Token, Value>) -> Self {
        Self {
            trie,
            queue: Vec::new(),
            start: LoudsNodeNum(1),
        }
    }

    pub fn labels<L>(self) -> impl Iterator<Item = L>
    where
        Token: Clone,
        L: TryFromTokens<Token>,
    {
        self.filter_map(|node_ref| node_ref.label().ok())
    }

    /// TODO: The real version would return &Value not Value.
    pub fn pairs<L>(self) -> impl Iterator<Item = (L, Value)>
    where
        Token: Clone,
        L: TryFromTokens<Token>,
        Value: Clone,
    {
        self.filter_map(|node_ref| {
            // node_ref.range().pair.ok()
            let label = node_ref.label().ok();
            let value = node_ref.value().cloned();
            label.zip(value)
        })
    }

    /// TODO: The real version would return &Value not Value.
    pub fn values<L>(self) -> impl Iterator<Item = Value>
    where
        Token: Clone,
        L: TryFromTokens<Token>,
        Value: Clone,
    {
        self.filter_map(|node_ref| node_ref.value().cloned())
    }

    pub fn suffixes<L>(self) -> impl Iterator<Item = L>
    where
        Token: Clone,
        L: TryFromTokens<Token>,
    {
        let start: LoudsNodeNum = self.start;
        self.filter_map(move |node_ref| {
            L::try_from_reverse_tokens(node_ref.range_from(start).map(|n| n.token().clone())).ok()
        })
    }
}

impl<'t, Token, Value> Iterator for StartsWith<'t, Token, Value> {
    type Item = NodeRef<'t, Token, Value>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut end: Option<LoudsNodeNum> = None;

        while end.is_none() {
            let Some(node) = self.queue.pop() else {
                break;
            };

            if node.0 == 1 {
                continue;
            }

            let children = self.trie.children_node_nums(node);
            self.queue.extend(children.rev());

            if self.trie.value(node).is_some() {
                end = Some(node);
            }
        }

        end.map(|end| NodeRef {
            trie: &self.trie,
            node_num: end,
        })
    }
}
