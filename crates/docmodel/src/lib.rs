use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: u32 = 1;

pub const MAX_DEPTH: usize = 256;
pub const MAX_NODES: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmDoc {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<PmNode>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmNode {
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<PmNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<PmMark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PmMark {
    #[serde(rename = "type")]
    pub mark_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attrs: Option<serde_json::Value>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DocumentError {
    #[error("корневой узел должен иметь тип \"doc\", получен \"{0}\"")]
    InvalidRoot(String),
    #[error("узел должен иметь непустой тип")]
    EmptyNodeType,
    #[error("текстовый узел \"{0}\" не может иметь дочерние узлы")]
    TextNodeWithContent(String),
    #[error("нетекстовый узел \"{0}\" не может содержать текст")]
    NonTextNodeWithText(String),
    #[error("текстовый узел должен содержать поле text")]
    EmptyTextNode,
    #[error("превышена максимальная глубина вложенности {MAX_DEPTH}")]
    TooDeep,
    #[error("превышено максимальное число узлов {MAX_NODES}")]
    TooManyNodes,
}

impl PmDoc {
    pub fn empty() -> Self {
        Self {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::paragraph()],
        }
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.node_type != "doc" {
            return Err(DocumentError::InvalidRoot(self.node_type.clone()));
        }

        let mut budget = NodeBudget {
            remaining: MAX_NODES,
        };

        for node in &self.content {
            validate_node(node, 1, &mut budget)?;
        }

        Ok(())
    }
}

impl PmNode {
    pub fn paragraph() -> Self {
        Self {
            node_type: "paragraph".to_owned(),
            attrs: None,
            content: Vec::new(),
            marks: Vec::new(),
            text: None,
        }
    }

    pub fn text(text: impl Into<String>) -> Self {
        Self {
            node_type: "text".to_owned(),
            attrs: None,
            content: Vec::new(),
            marks: Vec::new(),
            text: Some(text.into()),
        }
    }

    pub fn with_content(node_type: impl Into<String>, content: Vec<PmNode>) -> Self {
        Self {
            node_type: node_type.into(),
            attrs: None,
            content,
            marks: Vec::new(),
            text: None,
        }
    }
}

impl PmMark {
    pub fn new(mark_type: impl Into<String>) -> Self {
        Self {
            mark_type: mark_type.into(),
            attrs: None,
        }
    }
}

struct NodeBudget {
    remaining: usize,
}

fn validate_node(
    node: &PmNode,
    depth: usize,
    budget: &mut NodeBudget,
) -> Result<(), DocumentError> {
    if depth > MAX_DEPTH {
        return Err(DocumentError::TooDeep);
    }

    if node.node_type.is_empty() {
        return Err(DocumentError::EmptyNodeType);
    }

    if budget.remaining == 0 {
        return Err(DocumentError::TooManyNodes);
    }
    budget.remaining -= 1;

    if node.node_type == "text" {
        return match node.text {
            Some(_) if node.content.is_empty() => Ok(()),
            Some(_) => Err(DocumentError::TextNodeWithContent(node.node_type.clone())),
            None => Err(DocumentError::EmptyTextNode),
        };
    }

    if node.text.is_some() {
        return Err(DocumentError::NonTextNodeWithText(node.node_type.clone()));
    }

    for child in &node.content {
        validate_node(child, depth + 1, budget)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_document_is_valid_and_round_trips() {
        let doc = PmDoc::empty();
        doc.validate().expect("пустой документ должен быть валиден");

        let json = serde_json::to_value(&doc).expect("документ должен сериализоваться");
        assert_eq!(json["type"], "doc");
        assert_eq!(json["content"][0]["type"], "paragraph");

        let restored: PmDoc =
            serde_json::from_value(json).expect("документ должен десериализоваться");
        assert_eq!(restored, doc);
    }

    #[test]
    fn document_with_marks_round_trips() {
        let doc = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode {
                    marks: vec![PmMark::new("bold")],
                    ..PmNode::text("важно")
                }],
            )],
        };

        doc.validate()
            .expect("документ с разметкой должен быть валиден");

        let json = serde_json::to_string(&doc).expect("документ должен сериализоваться");
        let restored: PmDoc = serde_json::from_str(&json).expect("документ должен читаться");
        assert_eq!(restored, doc);
    }

    #[test]
    fn rejects_non_doc_root() {
        let doc = PmDoc {
            node_type: "paragraph".to_owned(),
            attrs: None,
            content: Vec::new(),
        };

        assert_eq!(
            doc.validate(),
            Err(DocumentError::InvalidRoot("paragraph".to_owned()))
        );
    }

    #[test]
    fn rejects_text_node_with_children() {
        let node = PmNode {
            content: vec![PmNode::text("вложенный")],
            ..PmNode::text("родитель")
        };
        let incomplete = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![node],
        };

        assert_eq!(
            incomplete.validate(),
            Err(DocumentError::TextNodeWithContent("text".to_owned()))
        );
    }

    #[test]
    fn rejects_text_on_non_text_node() {
        let doc = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode {
                text: Some("не туда".to_owned()),
                ..PmNode::paragraph()
            }],
        };

        assert_eq!(
            doc.validate(),
            Err(DocumentError::NonTextNodeWithText("paragraph".to_owned()))
        );
    }

    #[test]
    fn rejects_text_node_without_text() {
        let doc = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode {
                node_type: "text".to_owned(),
                ..PmNode::paragraph()
            }],
        };

        assert_eq!(doc.validate(), Err(DocumentError::EmptyTextNode));
    }

    #[test]
    fn rejects_empty_node_type() {
        let doc = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content("", Vec::new())],
        };

        assert_eq!(doc.validate(), Err(DocumentError::EmptyNodeType));
    }

    #[test]
    fn rejects_too_deep_documents() {
        let mut node = PmNode::paragraph();
        for _ in 0..=MAX_DEPTH {
            node = PmNode::with_content("blockquote", vec![node]);
        }
        let doc = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![node],
        };

        assert_eq!(doc.validate(), Err(DocumentError::TooDeep));
    }
}
