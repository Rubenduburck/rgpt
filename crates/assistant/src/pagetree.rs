use crate::{
    error::Error,
    textarea::{SessionAreaId, SessionTextArea},
};
use rgpt_types::message::Message;

#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub enum NodeId {
    #[default]
    Root,
    Node(u16),
}

impl From<NodeId> for String {
    fn from(id: NodeId) -> Self {
        match id {
            NodeId::Root => "root".to_string(),
            NodeId::Node(id) => format!("{}", id),
        }
    }
}

pub struct Root<'a> {
    pub nodes: Vec<Node<'a>>,
    pub active: NodeId,
    pub system_area: SessionTextArea<'a>,
    pub children: Vec<NodeId>,
}

impl<'a> Root<'a> {
    pub fn new(max_line_length: usize) -> Self {
        Root {
            nodes: vec![],
            active: NodeId::default(),
            system_area: SessionTextArea::new(SessionAreaId::System, &[], max_line_length),
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
        self.system_area.set_title("root > system".to_string());
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

    pub fn insert_messages(&mut self, parent: Option<NodeId>, messages: Vec<Message>) -> Result<NodeId, Error> {
        fn inner(
            tree: &mut Root,
            parent: Option<NodeId>,
            mut stack: Vec<Message>,
        ) -> Result<NodeId, Error> {
            if stack.is_empty() {
                return Ok(parent.unwrap_or(NodeId::Root));
            }
            let parent = parent.unwrap_or(NodeId::Root);
            let child = tree.insert_child(parent);
            let child_node = tree.get_mut(child).unwrap();
            let message = stack.pop().unwrap();

            if SessionAreaId::from(message.role) != SessionAreaId::User {
                return Err(Error::Generic("invalid message role".to_string()));
            }
            child_node.user_area.set_message(message);

            let message = if stack.is_empty() {
                return Ok(child);
            } else {
                stack.pop().unwrap()
            };

            if SessionAreaId::from(message.role) != SessionAreaId::Assistant {
                return Err(Error::Generic("invalid message role".to_string()));
            }

            child_node.assistant_area.set_message(message);

            inner(tree, Some(child), stack)
        }

        let mut messages = messages;
        let system_message = messages
            .iter()
            .find(|message| message.role == rgpt_types::message::Role::System);
        if let Some(system_message) = system_message {
            self.system_area.set_message(system_message.clone());
            messages.retain(|m| m.role != rgpt_types::message::Role::System);
        }
        messages.reverse();
        inner(self, parent, messages)
    }

    pub fn walk_up(&self, id: NodeId) -> Vec<NodeId> {
        let mut path = vec![];
        let mut id = id;
        while id != NodeId::Root {
            path.push(id);
            id = self.get(id).unwrap().parent;
        }
        path.push(NodeId::Root);
        path
    }

    pub fn next_id(&self) -> NodeId {
        NodeId::Node(self.nodes.len() as u16)
    }

    fn node_path_string(&self, node: NodeId) -> String {
        self.walk_up(node)
            .into_iter()
            .map(String::from)
            .rev()
            .collect::<Vec<_>>()
            .join(" > ")
    }

    pub fn insert_child(&mut self, parent: NodeId) -> NodeId {
        let id = self.next_id();
        let node = Node::new(id, parent, self.height(parent) + 1, self.system_area.max_line_length);
        self.nodes.push(node);
        let path_str = self.node_path_string(id);
        match parent {
            NodeId::Root => self.children.push(id),
            NodeId::Node(parent) => self.nodes[parent as usize].children.push(id),
        }
        let node = self.get_mut(id).unwrap();
        node.set_titles(path_str);
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
        match siblings
            .iter()
            .rev()
            .skip_while(|&&sibling| sibling != id)
            .nth(1)
        {
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
        Self::new(70)
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
    pub fn new(id: NodeId, parent: NodeId, height: u16, max_line_length: usize) -> Self {
        Node {
            id,
            user_area: SessionTextArea::new(SessionAreaId::User, &[], max_line_length),
            assistant_area: SessionTextArea::new(SessionAreaId::Assistant, &[], max_line_length),
            children: vec![],
            parent,
            height,
            active: None,
        }
    }

    pub fn set_titles(&mut self, path_str: String) {
        tracing::trace!("setting titles for node {:?}", self.id);
        self.user_area.set_title(format!("{} : user", path_str));
        self.assistant_area
            .set_title(format!("{} : assistant", path_str));
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
    use super::*;

    #[test]
    fn test_new_page_tree() {
        let tree = Root::default();
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.active, NodeId::Root);
    }

    #[test]
    fn test_insert_child() {
        let mut tree = Root::default();
        let child_id = tree.insert_child(NodeId::Root);
        assert_eq!(child_id, NodeId::Node(0));
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.nodes[0].parent, NodeId::Root);
    }

    #[test]
    fn test_activate() {
        let mut tree = Root::default();
        let child_id = tree.insert_child(NodeId::Root);
        tree.activate(child_id, SessionAreaId::User);
        assert_eq!(tree.active, child_id);
        assert_eq!(
            tree.get(child_id).unwrap().active,
            Some(SessionAreaId::User)
        );
    }

}
