use std::{collections::HashMap, sync::Arc};

use anyhow::{Context, Result};
use fluent_pipeline::{ConditionType, Edge, Node, Pipeline};
use serde::{Deserialize, Serialize};

use crate::{
    ai::{FluentRequest, FluentResponse},
    prelude::fluent::FluentAdapterType,
};

use super::{
    adapters::{commons::DefaultMerge, fluent::FluentAdapter},
    helpers::resolve_env_var,
};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct TransferData {
    #[serde(skip)]
    pub previous: Option<Arc<TransferData>>,
    pub value: TransferDataValue,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum TransferDataValue {
    #[default]
    Empty,
    Fluent(FluentRequest, FluentResponse),
    String(String),
    Number(f64),
    Json(serde_json::Value),
    #[serde(skip)]
    Merge(Vec<Arc<TransferData>>),
}

impl TransferData {
    pub fn string_value(&self) -> String {
        match &self.value {
            TransferDataValue::Empty => "".to_string(),
            TransferDataValue::String(s) => s.clone(),
            TransferDataValue::Number(n) => n.to_string(),
            TransferDataValue::Json(j) => j.to_string(),
            TransferDataValue::Fluent(_, response) => response.data.content.clone(),
            TransferDataValue::Merge(merge) => merge
                .iter()
                .map(|data| data.string_value())
                .collect::<Vec<String>>()
                .join("%%\n"),
        }
    }
    pub fn history_as_string(&self) -> String {
        let mut history = Vec::new();
        let mut current = self;
        // Traverse through the `previous` chain, collecting string values
        while let Some(prev) = &current.previous {
            history.push(prev.string_value());
            current = prev; // Move to the previous TransferData
        }
        // Reverse the order to get from first to last
        history.reverse();
        // Collect the current TransferData's string value last, as it's the most recent
        history.push(self.string_value());
        // Join all strings with a separator (e.g., "\n") and return
        history.join("\n")
    }
}

pub enum FluentMerge {
    Default(DefaultMerge),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PipelineConfig {
    pub nodes: Vec<NodeConfig>,
}
impl PipelineConfig {
    pub fn node(mut self, node: NodeConfig) -> Self {
        self.nodes.push(node);
        self
    }
}

impl TryFrom<&str> for PipelineConfig {
    type Error = anyhow::Error;
    fn try_from(config: &str) -> Result<Self> {
        let yaml: serde_yaml::Value = serde_yaml::from_str(config)?;
        let mut json = serde_json::to_value(yaml)?;
        resolve_env_var(&mut json)?;
        serde_json::from_value(json).context("Failed to parse pipeline config")
    }
}

impl TryFrom<PipelineConfig> for Pipeline<Arc<TransferData>> {
    type Error = anyhow::Error;
    fn try_from(mut config: PipelineConfig) -> Result<Self> {
        let mut pipeline = Pipeline::default();
        let mut connections = HashMap::new();
        while let Some(node) = config.nodes.pop() {
            connections.insert(node.name.clone(), node.connections);
            pipeline = match node.r#type {
                NodeType::Start => pipeline.add_node(&node.name, Node::Start)?,
                NodeType::End => pipeline.add_node(&node.name, Node::End)?,
                NodeType::Task(task) => match task {
                    Task::Fluent(adapter) => match adapter.r#type {
                        FluentAdapterType::OpenAIChat(request) => pipeline.add_node(
                            &node.name,
                            Node::Task(Arc::new(FluentAdapter {
                                append_history: adapter.append_history,
                                r#type: FluentAdapterType::OpenAIChat(request),
                            })),
                        )?,
                    },
                },
                NodeType::Split => pipeline.add_node(&node.name, Node::Split)?,
                NodeType::Join => pipeline.add_node(&node.name, Node::Join)?,
                NodeType::Decision => pipeline.add_node(&node.name, Node::Decision)?,
                NodeType::Merge(merge) => match merge {
                    Merge::Default(d) => pipeline.add_node(&node.name, Node::Merge(Arc::new(d)))?,
                },
            };
        }
        for (name, connections) in connections.drain() {
            for connection in connections {
                let edge = match connection.condition {
                    Condition::Fluent(adapter) => match adapter.r#type {
                        FluentAdapterType::OpenAIChat(request) => Edge::default().condition(
                            ConditionType::Eval(Arc::new(FluentAdapter {
                                append_history: connection.append_history,
                                r#type: FluentAdapterType::OpenAIChat(request),
                            })),
                        ),
                    },
                    Condition::AlwaysPass => Edge::default(),
                };
                pipeline = pipeline.connect(&name, &connection.to, edge)?;
            }
        }
        Ok(pipeline)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NodeConfig {
    pub name: String,
    pub r#type: NodeType,
    pub connections: Vec<NodeConnection>,
}
impl NodeConfig {
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }
    pub fn r#type(mut self, r#type: NodeType) -> Self {
        self.r#type = r#type;
        self
    }
    pub fn next(mut self, to: &str) -> Self {
        self.connections.push(NodeConnection {
            to: to.to_string(),
            condition: Default::default(),
            append_history: false,
        });
        self
    }
    pub fn next_if(mut self, to: &str, condition: Condition, append_history: bool) -> Self {
        self.connections.push(NodeConnection {
            to: to.to_string(),
            condition,
            append_history,
        });
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NodeConnection {
    pub to: String,
    pub condition: Condition,
    append_history: bool,
}
impl NodeConnection {
    pub fn to(mut self, to: &str) -> Self {
        self.to = to.to_string();
        self
    }
    pub fn condition(mut self, condition: Condition) -> Self {
        self.condition = condition;
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum NodeType {
    #[default]
    Start,
    Task(Task),
    Split,
    Merge(Merge),
    Decision,
    Join,
    End,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Task {
    Fluent(FluentAdapter),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Merge {
    Default(DefaultMerge),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum Condition {
    #[default]
    AlwaysPass,
    Fluent(FluentAdapter),
}
