use crate::{
    error::Error,
    session::{SessionAreaId, SessionTextArea},
};
use rgpt_types::message::Message;

pub type PageTreeNodeId = u16;

pub struct PageTree<'a> {
    pub nodes: Vec<PageTreeNode<'a>>,
    pub active: PageTreeNodeId,
}

impl<'a> PageTree<'a> {
    pub fn new() -> Self {
        PageTree {
            nodes: vec![PageTreeNode::new(0, 0, 0)],
            active: PageTreeNodeId::default(),
        }
    }

    pub fn root_id(&self) -> PageTreeNodeId {
        0
    }

    pub fn activate(&mut self, id: PageTreeNodeId, area: SessionAreaId) {
        if let Some(node) = self.get_mut(self.active) {
            node.inactivate();
            self.active = 0;
        }
        if let Some(node) = self.get_mut(id) {
            node.activate(area);
            self.active = id;
        }
    }

    pub fn root(&self) -> &PageTreeNode<'a> {
        self.get(self.root_id()).unwrap()
    }

    pub fn root_mut(&mut self) -> &mut PageTreeNode<'a> {
        self.get_mut(self.root_id()).unwrap()
    }

    pub fn insert_text_areas(
        &mut self,
        parent: Option<PageTreeNodeId>,
        text_areas: Vec<SessionTextArea<'a>>,
    ) -> Result<PageTreeNodeId, Error> {
        fn inner<'a>(
            tree: &mut PageTree<'a>,
            parent: Option<PageTreeNodeId>,
            mut stack: Vec<SessionTextArea<'a>>,
        ) -> Result<PageTreeNodeId, Error> {
            if stack.is_empty() {
                return Ok(tree.root_id());
            }
            let parent = parent.unwrap_or(tree.root_id());
            let child = tree.insert_child(parent);
            let child_node = tree.get_mut(child).unwrap();

            let next_area = stack.pop().unwrap();
            if next_area.id != SessionAreaId::User {
                return Err(Error::Generic("invalid area id".to_string()));
            }
            child_node.user_area = next_area;

            let next_area = if stack.is_empty() {
                return Ok(child);
            } else {
                stack.pop().unwrap()
            };
            if next_area.id != SessionAreaId::Assistant {
                return Err(Error::Generic("invalid area id".to_string()));
            }
            child_node.assistant_area = next_area;

            inner(tree, Some(child), stack)
        }
        inner(self, parent, text_areas.into_iter().rev().collect())
    }

    pub fn insert_child(&mut self, parent: PageTreeNodeId) -> PageTreeNodeId {
        let id = self.nodes.len() as PageTreeNodeId;
        let node = PageTreeNode::new(id, parent, self.height(parent) + 1);
        self.nodes.push(node);
        self.nodes[parent as usize].children.push(id);
        id
    }

    pub fn height(&self, id: PageTreeNodeId) -> u16 {
        self.get(id).map(|node| node.height).unwrap_or_default()
    }

    pub fn get(&self, id: PageTreeNodeId) -> Option<&PageTreeNode<'a>> {
        self.nodes.get(id as usize)
    }

    pub fn get_mut(&mut self, id: PageTreeNodeId) -> Option<&mut PageTreeNode<'a>> {
        self.nodes.get_mut(id as usize)
    }

    pub fn parent(&self, id: PageTreeNodeId) -> Option<&PageTreeNode<'a>> {
        self.get(id).and_then(|node| self.get(node.parent))
    }

    pub fn parent_mut(&mut self, id: PageTreeNodeId) -> Option<&mut PageTreeNode<'a>> {
        let parent_id = self.parent(id)?.id;
        self.get_mut(parent_id)
    }

    pub fn siblings(&self, id: PageTreeNodeId) -> (&[PageTreeNodeId], &[PageTreeNodeId]) {
        let parent_id = self.get(id).map(|node| node.parent).unwrap();
        let parent = self.get(parent_id).unwrap();
        if let Some(index) = parent.children.iter().position(|&child| child == id) {
            let (left, right) = parent.children.split_at(index);
            (left, &right[1..])
        } else {
            (&[], &[])
        }
    }

    pub fn siblings_mut(
        &mut self,
        id: PageTreeNodeId,
    ) -> (&mut [PageTreeNodeId], &mut [PageTreeNodeId]) {
        let parent_id = self.get(id).map(|node| node.parent).unwrap();
        let parent = self.get_mut(parent_id).unwrap();
        let index = parent
            .children
            .iter()
            .position(|&child| child == id)
            .unwrap();
        let (left, right) = parent.children.split_at_mut(index);
        let right = &mut right[1..];
        (left, right)
    }

    pub fn next_sibling(&self, id: PageTreeNodeId) -> Option<&PageTreeNode<'a>> {
        self.siblings(id).1.first().and_then(|&id| self.get(id))
    }

    pub fn next_sibling_mut(&mut self, id: PageTreeNodeId) -> Option<&mut PageTreeNode<'a>> {
        let next_sibling_id = self.siblings(id).1.first().copied()?;
        self.get_mut(next_sibling_id)
    }

    pub fn previous_sibling(&self, id: PageTreeNodeId) -> Option<&PageTreeNode<'a>> {
        self.siblings(id).0.last().and_then(|&id| self.get(id))
    }

    pub fn previous_sibling_mut(&mut self, id: PageTreeNodeId) -> Option<&mut PageTreeNode<'a>> {
        let previous_sibling_id = self.siblings(id).0.last().copied()?;
        self.get_mut(previous_sibling_id)
    }

    pub fn children(&self, id: PageTreeNodeId) -> Vec<&PageTreeNode<'a>> {
        self.get(id)
            .map(|node| {
                node.children
                    .iter()
                    .filter_map(|&id| self.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn parent_at_height(&self, id: PageTreeNodeId, height: u16) -> Option<&PageTreeNode<'a>> {
        let mut node = self.get(id)?;
        while node.height > height {
            node = self.get(node.parent)?;
        }
        Some(node)
    }

    pub fn collect_messages(&self, id: PageTreeNodeId, up_to: Option<u16>) -> Vec<Message> {
        let mut messages = vec![];
        let mut node = self.get(id).unwrap();
        let up_to = up_to.unwrap_or(self.root().height);
        while node.height > up_to {
            messages.push(Message::from(&node.assistant_area));
            messages.push(Message::from(&node.user_area));
            node = self.get(node.parent).unwrap();
        }
        messages.reverse();
        messages
    }
}

impl<'a> Default for PageTree<'a> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct PageTreeNode<'a> {
    pub id: PageTreeNodeId,
    pub user_area: SessionTextArea<'a>,
    pub assistant_area: SessionTextArea<'a>,
    pub children: Vec<PageTreeNodeId>,
    pub parent: PageTreeNodeId,
    pub height: u16,
    pub active: Option<SessionAreaId>,
}

impl<'a> PageTreeNode<'a> {
    pub fn new(id: PageTreeNodeId, parent: PageTreeNodeId, height: u16) -> Self {
        PageTreeNode {
            id,
            user_area: SessionTextArea::new(SessionAreaId::User, &[], 70),
            assistant_area: SessionTextArea::new(SessionAreaId::Assistant, &[], 70),
            children: vec![],
            parent,
            height,
            active: None,
        }
    }

    pub fn area(&self, id: SessionAreaId) -> &SessionTextArea<'a> {
        match id {
            SessionAreaId::User => &self.user_area,
            SessionAreaId::Assistant => &self.assistant_area,
            _ => panic!("invalid area id"),
        }
    }

    pub fn area_mut(&mut self, id: SessionAreaId) -> &mut SessionTextArea<'a> {
        match id {
            SessionAreaId::User => &mut self.user_area,
            SessionAreaId::Assistant => &mut self.assistant_area,
            _ => panic!("invalid area id"),
        }
    }

    pub fn activate(&mut self, area: SessionAreaId) {
        if let Some(active) = self.active {
            self.area_mut(active).inactivate();
        }
        if area == SessionAreaId::System {
            self.active = None;
            return;
        }
        self.area_mut(area).activate();
        self.active = Some(area);
    }

    pub fn inactivate(&mut self) {
        self.assistant_area.inactivate();
        self.user_area.inactivate();
        self.active = None
    }
}

#[cfg(test)]
mod tests {
    use rgpt_types::message::Role;

    use super::*;

    #[test]
    fn test_new_page_tree() {
        let tree = PageTree::new();
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.active, 0);
        assert_eq!(tree.root_id(), 0);
    }

    #[test]
    fn test_insert_child() {
        let mut tree = PageTree::new();
        let child_id = tree.insert_child(0);
        assert_eq!(child_id, 1);
        assert_eq!(tree.nodes.len(), 2);
        assert_eq!(tree.nodes[0].children, vec![1]);
    }

    #[test]
    fn test_activate() {
        let mut tree = PageTree::new();
        let child_id = tree.insert_child(0);
        tree.activate(child_id, SessionAreaId::User);
        assert_eq!(tree.active, child_id);
        assert_eq!(
            tree.nodes[child_id as usize].active,
            Some(SessionAreaId::User)
        );
    }

    #[test]
    fn test_insert_text_areas() {
        let mut tree = PageTree::new();
        let text_areas = vec![
            SessionTextArea::new(SessionAreaId::User, &[], 70),
            SessionTextArea::new(SessionAreaId::Assistant, &[], 70),
            SessionTextArea::new(SessionAreaId::User, &[], 70),
        ];
        let result = tree.insert_text_areas(None, text_areas);
        println!("{:?}", result);
        assert!(result.is_ok());
        assert_eq!(tree.nodes.len(), 3);
    }

    #[test]
    fn test_siblings() {
        let mut tree = PageTree::new();

        // Test case 1: Normal case (middle child)
        let child1 = tree.insert_child(0);
        let child2 = tree.insert_child(0);
        let child3 = tree.insert_child(0);
        let (left, right) = tree.siblings(child2);
        assert_eq!(left, &[child1]);
        assert_eq!(right, &[child3]);

        // Test case 2: First child
        let (left, right) = tree.siblings(child1);
        assert!(left.is_empty());
        assert_eq!(right, &[child2, child3]);

        // Test case 3: Last child
        let (left, right) = tree.siblings(child3);
        assert_eq!(left, &[child1, child2]);
        assert!(right.is_empty());

        // Test case 4: Only child
        let single_child = tree.insert_child(child1);
        let (left, right) = tree.siblings(single_child);
        assert!(left.is_empty());
        assert!(right.is_empty());

        // Test case 5: Root node (no siblings)
        let (left, right) = tree.siblings(tree.root_id());
        assert!(left.is_empty());
        assert!(right.is_empty());
    }

    #[test]
    fn test_collect_messages() {
        let messages = [
            Message {
                role: Role::User,
                content: "User message\n".to_string(),
            },
            Message {
                role: Role::Assistant,
                content: "Assistant message\n".to_string(),
            },
            Message {
                role: Role::User,
                content: "Another User message\n".to_string(),
            },
            Message {
                role: Role::Assistant,
                content: "Another Assistant message\n".to_string(),
            },
            Message {
                role: Role::User,
                content: "A last User message\n".to_string(),
            },
        ];
        let text_areas = messages
            .iter()
            .map(|m| {
                let id = SessionAreaId::from(m.role);
                let lines = m.content.lines().collect::<Vec<_>>();
                SessionTextArea::new(id, lines.as_slice(), 100)
            })
            .collect();
        let mut tree = PageTree::new();
        tree.insert_text_areas(None, text_areas).unwrap();
        let collected = tree.collect_messages(1, None);
        println!("original {:?}", messages);
        println!("collected {:?}", collected);
        for (left, right) in messages.iter().zip(collected.iter()) {
            assert_eq!(left.role, right.role);
            assert_eq!(left.content, right.content);
        }
    }
}
