use crate::{
    error::Error,
    session::{SessionAreaId, SessionTextArea},
};
use rgpt_types::message::Message;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeId {
    #[default]
    Root,
    Node(u16),
}

pub struct Root<'a> {
    pub nodes: Vec<Node<'a>>,
    pub active: NodeId,
    pub system_area: SessionTextArea<'a>,
    pub children: Vec<NodeId>,
}

impl<'a> Root<'a> {
    pub fn new() -> Self {
        Root {
            nodes: vec![],
            active: NodeId::default(),
            system_area: SessionTextArea::new(SessionAreaId::System, &[], 70),
            children: vec![],
        }
    }

    // FIXME: this is messy
    pub fn activate(&mut self, id: NodeId, area: SessionAreaId) {
        if let Some(node) = self.get_mut(self.active) {
            node.inactivate();
            self.active = NodeId::Root;
        }
        self.system_area.inactivate();
        match area {
            SessionAreaId::System => {
                self.system_area.activate();
            }
            SessionAreaId::Assistant => {
                let success = {
                    if let Some(node) = self.get_mut(id) {
                        if node.assistant_area.is_empty() {
                            false
                        } else {
                            node.activate(SessionAreaId::Assistant);
                            self.active = id;
                            true
                        }
                    } else {
                        false
                    }
                };
                if !success {
                    if let Some(parent) = self.parent_mut(id) {
                        parent.activate(SessionAreaId::Assistant);
                        self.active = parent.id;
                    }
                }
            }
            SessionAreaId::User => {
                if let Some(node) = self.get_mut(id) {
                    node.activate(SessionAreaId::User);
                    self.active = id;
                }
            }
        }
    }

    // TODO: make this order-agnostic
    pub fn insert_text_areas(
        &mut self,
        parent: Option<NodeId>,
        mut text_areas: Vec<SessionTextArea<'a>>,
    ) -> Result<NodeId, Error> {
        fn inner<'a>(
            tree: &mut Root<'a>,
            parent: Option<NodeId>,
            mut stack: Vec<SessionTextArea<'a>>,
        ) -> Result<NodeId, Error> {
            tracing::trace!("stack: {:?}", stack);
            if stack.is_empty() {
                return Ok(parent.unwrap_or(NodeId::Root));
            }
            let parent = parent.unwrap_or(NodeId::Root);
            let child = tree.insert_child(parent);
            tracing::trace!("inserting child {:?} under parent {:?}", child, parent);
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
        let system_area = text_areas
            .iter()
            .find(|area| area.id == SessionAreaId::System);
        if let Some(system_area) = system_area {
            self.system_area = system_area.clone();
        }
        text_areas.reverse();
        text_areas.pop();

        inner(self, parent, text_areas)
    }

    pub fn next_id(&self) -> NodeId {
        NodeId::Node(self.nodes.len() as u16)
    }

    pub fn insert_child(&mut self, parent: NodeId) -> NodeId {
        let id = self.next_id();
        let node = Node::new(id, parent, self.height(parent) + 1);
        self.nodes.push(node);
        match parent {
            NodeId::Root => self.children.push(id),
            NodeId::Node(parent) => self.nodes[parent as usize].children.push(id),
        }
        id
    }

    pub fn get_system_area(&self) -> &SessionTextArea<'a> {
        &self.system_area
    }

    pub fn get_system_area_mut(&mut self) -> &mut SessionTextArea<'a> {
        &mut self.system_area
    }

    pub fn height(&self, id: NodeId) -> u16 {
        match id {
            NodeId::Root => 0,
            NodeId::Node(id) => self.nodes[id as usize].height,
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node<'a>> {
        match id {
            NodeId::Root => None,
            NodeId::Node(id) => self.nodes.get(id as usize),
        }
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node<'a>> {
        match id {
            NodeId::Root => None,
            NodeId::Node(id) => self.nodes.get_mut(id as usize),
        }
    }

    pub fn parent(&self, id: NodeId) -> Option<&Node<'a>> {
        self.get(id).and_then(|node| self.get(node.parent))
    }

    pub fn parent_mut(&mut self, id: NodeId) -> Option<&mut Node<'a>> {
        let parent_id = self.parent(id)?.id;
        self.get_mut(parent_id)
    }

    pub fn siblings(&self, id: NodeId) -> &[NodeId] {
        match self.parent(id) {
            Some(parent) => parent.children.as_slice(),
            None => self.children.as_slice(),
        }
    }

    pub fn siblings_mut(&mut self, id: NodeId) -> &mut [NodeId] {
        let parent_id = self.get(id).map(|node| node.parent).unwrap_or(NodeId::Root);
        match parent_id {
            NodeId::Root => &mut self.children,
            NodeId::Node(parent_id) => &mut self.nodes[parent_id as usize].children,
        }
    }

    pub fn next_sibling(&self, id: NodeId) -> Option<&Node<'a>> {
        let siblings = self.siblings(id);
        match siblings.iter().skip_while(|&&sibling| sibling != id).nth(1) {
            Some(&id) => self.get(id),
            None => siblings.first().and_then(|&id| self.get(id)),
        }
    }

    pub fn previous_sibling(&self, id: NodeId) -> Option<&Node<'a>> {
        let siblings = self.siblings(id);
        match siblings.iter().rev().skip_while(|&&sibling| sibling != id).nth(1) {
            Some(&id) => self.get(id),
            None => siblings.last().and_then(|&id| self.get(id)),
        }
    }

    pub fn children(&self, id: NodeId) -> Vec<&Node<'a>> {
        self.get(id)
            .map(|node| {
                node.children
                    .iter()
                    .filter_map(|&id| self.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn parent_at_height(&self, id: NodeId, height: u16) -> Option<&Node<'a>> {
        let mut node = self.get(id)?;
        while node.height > height {
            node = self.get(node.parent)?;
        }
        Some(node)
    }

    pub fn get_node_messages(&self, id: NodeId) -> Vec<Message> {
        match id {
            NodeId::Root => self.system_area.message().into_iter().collect(),
            id @ NodeId::Node(_) => self.get(id).map(|node| node.messages()).unwrap_or_default(),
        }
    }

    pub fn collect_messages(&self, id: NodeId, down_to: Option<u16>) -> Vec<Message> {
        tracing::trace!(
            "collecting messages from node {:?} down to {:?}",
            id,
            down_to
        );
        let mut messages = vec![];
        let down_to = down_to.unwrap_or(0);
        let mut height = self.height(id);
        let mut id = id;
        while height > down_to {
            messages.extend(self.get_node_messages(id));
            id = self.get(id).map(|node| node.parent).unwrap_or(NodeId::Root);
            height -= 1;
        }
        messages.reverse();
        if messages.last().map(|m| m.role) == Some(rgpt_types::message::Role::Assistant) {
            messages.pop();
        }
        messages
    }
}

impl<'a> Default for Root<'a> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Node<'a> {
    pub id: NodeId,
    pub user_area: SessionTextArea<'a>,
    pub assistant_area: SessionTextArea<'a>,
    pub children: Vec<NodeId>,
    pub parent: NodeId,
    pub height: u16,
    pub active: Option<SessionAreaId>,
}

impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageTreeNode")
            .field("id", &self.id)
            .field("children", &self.children)
            .field("parent", &self.parent)
            .field("height", &self.height)
            .field("active", &self.active)
            .finish()
    }
}

impl<'a> Node<'a> {
    pub fn new(id: NodeId, parent: NodeId, height: u16) -> Self {
        Node {
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

    pub fn messages(&self) -> Vec<Message> {
        match (self.user_area.message(), self.assistant_area.message()) {
            (Some(user), Some(assistant)) => vec![assistant, user],
            (Some(user), None) => vec![user],
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use rgpt_types::message::Role;

    use super::*;

    #[test]
    fn test_new_page_tree() {
        let tree = Root::new();
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.active, NodeId::Root);
    }

    #[test]
    fn test_insert_child() {
        let mut tree = Root::new();
        let child_id = tree.insert_child(NodeId::Root);
        assert_eq!(child_id, NodeId::Node(0));
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].parent, NodeId::Root);
    }

    #[test]
    fn test_activate() {
        let mut tree = Root::new();
        let child_id = tree.insert_child(NodeId::Root);
        tree.activate(child_id, SessionAreaId::User);
        assert_eq!(tree.active, child_id);
        assert_eq!(
            tree.get(child_id).unwrap().active,
            Some(SessionAreaId::User)
        );
    }

    #[test]
    fn test_insert_text_areas() {
        let mut tree = Root::new();
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
        let mut tree = Root::new();
        tree.insert_text_areas(None, text_areas).unwrap();
        let collected = tree.collect_messages(NodeId::Node(1), None);
        println!("original {:?}", messages);
        println!("collected {:?}", collected);
        for (left, right) in messages.iter().zip(collected.iter()) {
            assert_eq!(left.role, right.role);
            assert_eq!(left.content, right.content);
        }
    }
}
