#[cfg(test)]
pub mod tests_common;

use anyhow::Result;
use async_trait::async_trait;
use futures::lock::Mutex;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use std::collections::{HashMap, VecDeque};
use std::fmt::{self, Debug, Display};
use std::sync::Arc;
use std::vec;
use tracing::debug;

#[async_trait]
pub trait Executable<T>: Send + Sync {
    async fn execute(&self, input: Option<&T>) -> Result<Option<T>>;
}

#[async_trait]
pub trait Mergeable<T>: Send + Sync {
    async fn merge(&self, input: &[Option<&T>]) -> Result<T>;
}

#[async_trait]
pub trait Evaluator<T>: Send + Sync {
    async fn evaluate(&self, input: Option<&T>) -> Result<bool>;
}

#[async_trait]
pub trait InputAdapter<T>: Send + Sync {
    async fn adapt(&self, input: Option<&T>) -> Result<Option<T>>;
}

impl<T> Display for Node<T>
where
    T: Clone + Debug + Send + Sync + Default,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Start => write!(f, "Start Node"),
            Node::Split => write!(f, "Split Node"),
            Node::Task { .. } => write!(f, "Task Node"),
            Node::Merge { .. } => write!(f, "Merge Node"),
            Node::Decision { .. } => write!(f, "Conditional Node"),
            Node::Join => write!(f, "Join Node"),
            Node::End => write!(f, "End Node"),
        }
    }
}

/// Represents the various types of nodes that can be used in a processing pipeline.
/// Each node type dictates how tokens are processed and routed through the pipeline.
///
/// # Type Parameters
/// - `T`: The type of data that flows through the pipeline, encapsulated in tokens.
///
/// # Variants
///
///
pub enum Node<T> {
    /// The starting point of the pipeline.
    ///
    /// - **Input:** No input is allowed for this node.
    /// - **Output:** Allows only one outgoing edge.
    /// - **Constraints:** There can only be one `Start` node in the entire pipeline.
    ///
    /// Typically, this node initializes the first token with the initial data
    /// provided to the pipeline and passes it to the next node.
    Start,

    /// Splits a single token into multiple tokens, creating parallel execution paths.
    ///
    /// - **Input:** Allows exactly one incoming edge.
    /// - **Output:** Allows multiple outgoing edges.
    ///
    /// The input token is cloned, and a new token is sent along each outgoing edge
    ///
    /// Useful for scenarios where parallel tasks need to be executed simultaneously,
    Split,

    /// Represents a task that processes a token's data and produces a single output.
    ///
    /// - **Input:** Allows exactly one incoming edge.
    /// - **Output:** Allows exactly one outgoing edge.
    /// - **Behavior:** The task is executed using the token's input data, and the
    ///   output is encapsulated in a new token that is passed to the next node.
    ///
    /// The task logic is encapsulated in the `Executable<T>` trait, allowing for
    /// flexible and reusable task implementations.
    Task(Arc<dyn Executable<T>>),

    /// Merges multiple tokens into a single token, typically after a split operation.
    ///
    /// - **Input:** Allows multiple incoming edges.
    /// - **Output:** Allows exactly one outgoing edge.
    /// - **Behavior:** The inputs from all incoming tokens are combined using the
    ///   logic defined in the `Mergeable<T>` trait, producing a single output.
    ///
    /// This node is essential for synchronizing parallel branches back into a single
    /// execution path.
    Merge(Arc<dyn Mergeable<T>>),

    /// Conditionally routes a token to one of several possible next nodes based on the input data.
    ///
    /// - **Input:** Allows exactly one incoming edge.
    /// - **Output:** Allows multiple outgoing edges.
    /// - **Conditions:** Each outgoing edge can have an associated condition defined by
    ///   the `ConditionType<T>`. At least one edge must be of type `ConditionType::Pass`,
    ///   ensuring that a token can always proceed.
    ///
    /// Evaluates conditions (defined using the `Evaluator<T>` trait) on the input data
    /// and routes the token to the first matching edge. The last edge should always act as the
    /// default route if none of the conditions match.
    ///
    /// Ideal for decision-making scenarios where the token's path through the pipeline
    /// depends on its data.
    Decision,

    /// Joins multiple incoming tokens and passes all of them forward.
    ///
    /// - **Input:** Allows multiple incoming edges.
    /// - **Output:** Allows exactly one outgoing edge.
    /// - **Behavior:** Unlike `Merge`, this node does not combine data but simply forwards all incoming
    ///   tokens to the next node. This node is typically used after a decision or a split operation.
    ///
    /// Useful when multiple parallel branches need to converge at a single point without merging data.
    Join,

    /// Marks the endpoint of the pipeline.
    ///
    /// - **Input:** Allows exactly one incoming edge.
    /// - **Output:** No outgoing edges are allowed.
    /// - **Constraints:** There can only be one `End` node in the entire pipeline.
    ///
    /// The pipeline's execution terminates when a token reaches this node.
    End,
}
/// Represents a connection between two nodes in the pipeline, along with any associated
/// conditions and data transformations.
///
/// # Type Parameters
/// - `T`: The type of data that flows through the pipeline, encapsulated in tokens.
///
/// # Fields
#[derive(Default)]
pub struct Edge<T> {
    /// The index of the output from the current node that this edge represents.
    pub output_index: usize,

    /// The index of the input to the next node that this edge connects to.
    pub input_index: usize,

    /// The condition that must be met for this edge to be traversed.
    /// If `Pass`, the edge is always traversed. If `Eval`, the condition
    /// is evaluated using the `Evaluator<T>` trait.
    ///
    /// # ConditionType
    /// - `Pass`: The default condition. This edge will always be traversed.
    /// - `Eval`: A conditional evaluation using an implementation of `Evaluator<T>`.
    pub condition: ConditionType<T>,

    /// An optional adapter that can transform the token's input before it is passed
    /// to the next node. If provided, the adapter will be applied to the token's input
    /// after the condition is evaluated (if any) and before the token is forwarded.
    pub adapter: Option<Arc<dyn InputAdapter<T>>>,
}

/// Represents the condition type associated with an edge. It determines whether
/// a token should traverse the edge.
///
/// # Variants
/// - `Pass`: Always allows traversal of the edge.
/// - `Eval`: Evaluates a condition using the provided `Evaluator<T>` implementation.
#[derive(Default)]
pub enum ConditionType<T> {
    /// The default condition that always passes. Used when no evaluation is needed.
    #[default]
    Pass,

    /// A condition that evaluates a token's data using a custom `Evaluator<T>` implementation.
    Eval(Arc<dyn Evaluator<T>>),
}

impl<T> Edge<T> {
    /// Sets a condition for this edge.
    ///
    /// # Parameters
    /// - `condition`: The condition that must be met for this edge to be traversed.
    ///
    /// # Returns
    /// Returns a modified `Edge` with the specified condition.
    pub fn condition(mut self, condition: ConditionType<T>) -> Self {
        self.condition = condition;
        self
    }

    /// Sets an adapter for this edge to transform the token's input before forwarding it.
    ///
    /// # Parameters
    /// - `adapter`: An adapter that modifies the token's input before it is passed to the next node.
    ///
    /// # Returns
    /// Returns a modified `Edge` with the specified adapter.
    pub fn adapter(mut self, adapter: Arc<dyn InputAdapter<T>>) -> Self {
        self.adapter = Some(adapter);
        self
    }
}

#[derive(Debug)]
pub enum TokenStatus {
    Started,
    Completed,
    Failed,
    Aborted,
}
#[derive(Debug, Clone)]
pub struct Token<T> {
    pub id: String,
    pub split_stack: Vec<(String, usize)>,
    pub node_index: NodeIndex,
    pub input: Option<T>,
}

type MergingsMap<T> = HashMap<(String, NodeIndex), Vec<Token<T>>>;

#[derive(Default)]
pub struct Pipeline<T> {
    graph: DiGraph<Node<T>, Edge<T>>,
    start_node_index: Option<NodeIndex>,
    end_node_index: Option<NodeIndex>,
    mergings: Arc<Mutex<MergingsMap<T>>>,
    name_to_index: HashMap<String, NodeIndex>,
    index_to_name: HashMap<NodeIndex, String>,
    output_edge_index: HashMap<NodeIndex, usize>,
    input_edge_counter: HashMap<NodeIndex, usize>,
}

impl<T> Pipeline<T>
where
    T: Clone + Debug,
{
    pub async fn run(&self, value: Option<T>) -> Result<Vec<Token<T>>> {
        let token = self
            .start_node_index
            .map(|index| Token {
                id: uuid::Uuid::new_v4().to_string(),
                split_stack: vec![],
                node_index: index,
                input: value,
            })
            .ok_or_else(|| anyhow::anyhow!("No start node found"))?;

        let end = self
            .end_node_index
            .ok_or_else(|| anyhow::anyhow!("No end node found"))?;

        let mut finished_tokens = vec![];
        let mut next_tokens = VecDeque::from(vec![token]);

        while let Some(token) = next_tokens.pop_front() {
            let node = self
                .graph
                .node_weight(token.node_index)
                .ok_or_else(|| anyhow::anyhow!("Node not found"))?;
            for token in self.execute(token, node).await? {
                if token.node_index == end {
                    finished_tokens.push(token);
                } else {
                    next_tokens.push_back(token);
                }
            }
        }
        Ok(finished_tokens)
    }

    async fn execute(&self, token: Token<T>, node: &Node<T>) -> Result<Vec<Token<T>>> {
        //Match the node type and execute the corresponding logic
        let next_tokens: Vec<Token<T>> = match node {
            Node::Start => self.process_start(token, node)?,
            Node::Split => self.process_split(token, node).await?,
            //Execute the task and create a new token
            Node::Task(execute) => self.process_task(token, node, execute).await?,
            Node::Merge(merge) => self.process_merge(token, node, merge).await?,
            Node::Decision => self.process_decision(token, node).await?,
            Node::Join => self.process_join(token, node).await?,
            Node::End => return Err(anyhow::anyhow!("End node reached")),
        };
        Ok(next_tokens)
    }

    pub fn add_node(mut self, name: &str, node: Node<T>) -> anyhow::Result<Self> {
        //Adding the node to the graph and the internal map
        if self
            .name_to_index
            .insert(name.to_string(), self.graph.add_node(node))
            .is_some()
        {
            return Err(anyhow::anyhow!("Node with name {} already exists", name));
        }

        //Getting the node index from the internal map
        let node_index = self.name_to_index.get(name).ok_or_else(|| {
            anyhow::anyhow!("Node not found in internal map after adding: {}", name)
        })?;
        self.index_to_name.insert(*node_index, name.to_string());

        //Getting the node from the graph
        let node = self
            .graph
            .node_weight(*node_index)
            .ok_or_else(|| anyhow::anyhow!("Node not found in the graph after adding: {}", name))?;

        //Checking if the node is a start or end node
        match node {
            Node::Start => {
                if self.start_node_index.is_some() {
                    return Err(anyhow::anyhow!("Start node already exists"));
                } else {
                    self.start_node_index = Some(*node_index);
                }
            }
            Node::End => {
                if self.end_node_index.is_some() {
                    return Err(anyhow::anyhow!("End node already exists"));
                } else {
                    self.end_node_index = Some(*node_index);
                }
            }
            _ => {}
        }

        //Return the pipeline
        Ok(self)
    }

    pub fn connect(mut self, from: &str, to: &str, mut info: Edge<T>) -> anyhow::Result<Self> {
        //Getting the "from" node index from the internal map
        let from = *self
            .name_to_index
            .get(from)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", from))?;

        //Getting the output index from the internal map
        info.output_index = *self
            .output_edge_index
            .entry(from)
            .and_modify(|e| *e += 1)
            .or_insert(0);

        //Getting the "to" node index from the internal map
        let to = *self
            .name_to_index
            .get(to)
            .ok_or_else(|| anyhow::anyhow!("Node not found: {}", to))?;
        //Getting the input index from the internal map
        info.input_index = *self
            .input_edge_counter
            .entry(to)
            .and_modify(|e| *e += 1)
            .or_insert(0);

        //Adding the edge to the graph
        self.graph.add_edge(from, to, info);

        //Return the pipeline
        Ok(self)
    }
    pub fn validate(self) -> anyhow::Result<Self> {
        // Ensure there is exactly one Start node
        if self.start_node_index.is_none() {
            return Err(anyhow::anyhow!(
                "Pipeline must have exactly one Start node, but none was found."
            ));
        }
        // Ensure there is exactly one End node
        if self.end_node_index.is_none() {
            return Err(anyhow::anyhow!(
                "Pipeline must have exactly one End node, but none was found."
            ));
        }

        for node_index in self.graph.node_indices() {
            let node = self.graph.node_weight(node_index);
            let incoming_edges = self
                .graph
                .edges_directed(node_index, Direction::Incoming)
                .count();
            let outgoing_edges = self
                .graph
                .edges_directed(node_index, Direction::Outgoing)
                .count();

            //Ensure node has no input edge
            if let Some(Node::Start) = node {
                if incoming_edges != 0 {
                    return Err(anyhow::anyhow!(
                        "Node {:?} must have exactly 0 incoming edges, but found {}",
                        self.index_to_name.get(&node_index),
                        incoming_edges
                    ));
                }
            }
            //Ensure node has exactly 1 incoming edge
            if let Some(Node::Task(_) | Node::Decision | Node::End | Node::Split) = node {
                if incoming_edges != 1 {
                    return Err(anyhow::anyhow!(
                        "Node {:?} must have exactly 1 incoming edge, but found {}",
                        self.index_to_name.get(&node_index),
                        incoming_edges
                    ));
                }
            }
            //Ensure node has 1 ore more incoming edges
            if let Some(Node::Merge(_) | Node::Join) = node {
                if incoming_edges < 1 {
                    return Err(anyhow::anyhow!(
                        "Node {:?} must have at least 1 incoming edge, but found {}",
                        self.index_to_name.get(&node_index),
                        incoming_edges
                    ));
                }
            }
            // Ensure node has exactly 1 outgoing edge
            if let Some(Node::Start | Node::Task(_) | Node::Join | Node::Merge(_)) = node {
                if outgoing_edges != 1 {
                    return Err(anyhow::anyhow!(
                        "Node {:?} must have exactly 1 outgoing edge, but found {}",
                        self.index_to_name.get(&node_index),
                        outgoing_edges
                    ));
                }
            }
            // Ensure node has 1 ore more outgoing edges
            if let Some(Node::Decision | Node::Split) = node {
                if outgoing_edges < 1 {
                    return Err(anyhow::anyhow!(
                        "Node {:?} must have at least 1 outgoing edge, but found {}",
                        self.index_to_name.get(&node_index),
                        outgoing_edges
                    ));
                }
            }
            //Ensure each Split and Decision node has at least one outgoing edge with ConditionType::Pass
            if let Some(Node::Split | Node::Decision) = node {
                let has_pass_edge = self
                    .graph
                    .edges_directed(node_index, Direction::Outgoing)
                    .any(|edge| matches!(edge.weight().condition, ConditionType::Pass));
                if !has_pass_edge {
                    return Err(anyhow::anyhow!(
                        "Node {:?} must have at least one outgoing edge with ConditionType::Pass",
                        self.index_to_name.get(&node_index)
                    ));
                }
            }
        }
        Ok(self)
    }

    fn process_start(&self, token: Token<T>, _node: &Node<T>) -> Result<Vec<Token<T>>> {
        let mut next_tokens = vec![];
        let input = token.input.clone();
        let next_node = self
            .graph
            .neighbors_directed(token.node_index, Direction::Outgoing)
            .next()
            .ok_or_else(|| anyhow::anyhow!("No outgoing edge found"))?;
        next_tokens.push(Token {
            id: uuid::Uuid::new_v4().to_string(),
            split_stack: vec![],
            node_index: next_node,
            input,
        });
        Ok(next_tokens)
    }
    async fn process_split(&self, token: Token<T>, _node: &Node<T>) -> Result<Vec<Token<T>>> {
        let mut next_tokens = vec![];
        //Clone the input
        let input = token.input.clone();
        let from_index = token.node_index;
        //Get the next nodes
        let next_nodes = self.get_next_nodes(&token, from_index, &input).await?;
        //Prepare the children parent stack
        let mut split_parent_stack = token.split_stack.clone();
        split_parent_stack.push((
            token.id.clone(),
            next_nodes.iter().filter(|(_, allowed, _)| *allowed).count(),
        ));
        for (to_index, _, _) in next_nodes {
            let input = self
                .apply_transition_input_adapter(from_index, to_index, &input)
                .await?;
            next_tokens.push(Token {
                node_index: to_index,
                id: uuid::Uuid::new_v4().to_string(),
                split_stack: split_parent_stack.clone(),
                input: input.clone(),
            });
        }
        Ok(next_tokens)
    }

    async fn process_task(
        &self,
        token: Token<T>,
        _node: &Node<T>,
        execute: &Arc<dyn Executable<T>>,
    ) -> Result<Vec<Token<T>>> {
        let output = execute.execute(token.input.as_ref()).await?;
        let from_node = token.node_index;
        let to_node = self.get_connected_node(&token)?;
        let input = self
            .apply_transition_input_adapter(from_node, to_node, &output)
            .await?;
        let next_token = Token {
            id: uuid::Uuid::new_v4().to_string(),
            split_stack: token.split_stack,
            node_index: to_node,
            input,
        };
        Ok(vec![next_token])
    }

    async fn process_merge(
        &self,
        token: Token<T>,
        _node: &Node<T>,
        merge: &Arc<dyn Mergeable<T>>,
    ) -> Result<Vec<Token<T>>> {
        let mut next_tokens = vec![];

        let from_node = token.node_index;
        let to_node = self.get_connected_node(&token)?;

        //Create a new parent stack from the token
        let mut parent_stack = token.split_stack.clone();

        //Get the parent id and the number of Splits from the parent stack
        let (parent_id, splits) = parent_stack.pop().ok_or_else(|| {
            anyhow::anyhow!("No previous Split found for merge node: {}", token.id)
        })?;

        //Get the mergings map
        let mut mergings = self.mergings.lock().await;

        //Create a key for the mergings map
        let key = (parent_id.clone(), token.node_index);

        //Get the entry from the mergings map. Create one if it is the first token
        let entry = mergings.entry(key.clone()).or_insert_with(Vec::new);
        debug!(
            "Node_Index: {}, Splits: {}, Processed: {}",
            token.node_index.index(),
            splits,
            entry.len()
        );

        //Get the node_index from the token
        let node_index = token.node_index;

        //Push the token to the entry
        entry.push(token);

        //Check if all tokens have arrived
        if entry.len() == splits {
            //Get the values from the entry
            let values: Vec<Option<&T>> = entry.iter().map(|t| t.input.as_ref()).collect();

            //Merge the values
            let merged_value = Some(merge.merge(values.as_slice()).await?);
            let input = self
                .apply_transition_input_adapter(from_node, to_node, &merged_value)
                .await?;

            //Create a new token with the merged value and push it to the next tokens
            next_tokens.push(Token {
                id: uuid::Uuid::new_v4().to_string(),
                split_stack: parent_stack,
                node_index: self
                    .graph
                    .neighbors_directed(node_index, Direction::Outgoing)
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No outgoing edge found"))?,
                input,
            });

            //Remove the entry from the mergings map
            mergings.remove(&key);
        } else {
            debug!(
                "Waiting for all tokens to arrive for last Split : {}",
                parent_id
            );
        }
        Ok(next_tokens)
    }
    async fn process_decision(&self, token: Token<T>, _node: &Node<T>) -> Result<Vec<Token<T>>> {
        let mut next_tokens = vec![];
        if token.input.is_none() {
            return Err(anyhow::anyhow!("No value found for conditional node"));
        }
        let from_node = token.node_index;
        let next_nodes = self.get_next_nodes(&token, from_node, &token.input).await?;
        for (to_node, is_allowed, _) in next_nodes {
            if is_allowed {
                next_tokens.push(Token {
                    id: uuid::Uuid::new_v4().to_string(),
                    split_stack: token.split_stack.clone(),
                    node_index: to_node,
                    input: self
                        .apply_transition_input_adapter(from_node, to_node, &token.input)
                        .await?,
                });
                break; // Only one path is allowed
            }
        }
        Ok(next_tokens)
    }

    async fn process_join(&self, token: Token<T>, _node: &Node<T>) -> Result<Vec<Token<T>>> {
        //Just move the token to the next node
        Ok(vec![Token {
            id: uuid::Uuid::new_v4().to_string(),
            node_index: self.get_connected_node(&token)?,
            split_stack: token.split_stack,
            input: token.input.clone(),
        }])
    }

    fn get_connected_node(
        &self,
        token: &Token<T>,
    ) -> std::result::Result<NodeIndex, anyhow::Error> {
        self.graph
            .neighbors_directed(token.node_index, Direction::Outgoing)
            .next()
            .ok_or_else(|| anyhow::anyhow!("No outgoing edge found"))
    }

    async fn get_next_nodes(
        &self,
        token: &Token<T>,
        from_index: NodeIndex,
        input: &Option<T>,
    ) -> Result<Vec<(NodeIndex, bool, usize)>, anyhow::Error> {
        let next_nodes = self
            .graph
            .neighbors_directed(token.node_index, Direction::Outgoing)
            .collect::<Vec<_>>();
        let mut allowed_nodes = vec![];
        for node in next_nodes.iter() {
            let edge = self.get_unique_output_edge(from_index, *node)?;
            allowed_nodes.push((*node, is_allowed(edge, input).await?, edge.output_index))
        }
        allowed_nodes.sort_by_key(|(_, _, index)| *index);
        Ok(allowed_nodes)
    }
    async fn apply_transition_input_adapter(
        &self,
        from_index: NodeIndex,
        to_index: NodeIndex,
        input: &Option<T>,
    ) -> Result<Option<T>, anyhow::Error>
    where
        T: Clone,
    {
        let edge = self.get_unique_output_edge(from_index, to_index)?;
        let input = if let Some(adapter) = edge.adapter.as_ref() {
            adapter.adapt(input.as_ref()).await?
        } else {
            input.clone()
        };
        Ok(input)
    }

    fn get_unique_output_edge(
        &self,
        from_index: NodeIndex,
        to_index: NodeIndex,
    ) -> Result<&Edge<T>, anyhow::Error> {
        let next_edges = self
            .graph
            .edges_connecting(from_index, to_index)
            .collect::<Vec<_>>();

        if next_edges.is_empty() {
            return Err(anyhow::anyhow!(
                "No edge found between {} and {}",
                from_index.index(),
                to_index.index()
            ));
        }

        if next_edges.len() > 1 {
            return Err(anyhow::anyhow!(
                "Multiple edges found between {} and {}",
                from_index.index(),
                to_index.index()
            ));
        }

        Ok(next_edges[0].weight())
    }
}

async fn is_allowed<T>(edge: &Edge<T>, input: &Option<T>) -> Result<bool, anyhow::Error> {
    match edge.condition {
        ConditionType::Pass => Ok(true),
        ConditionType::Eval(ref evaluator) => evaluator.evaluate(input.as_ref()).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use async_trait::async_trait;
    use std::sync::Arc;

    enum MathOp {
        Add(i32),
        Multiply(i32),
        Identity,
    }

    enum Filter {
        LessThan(i32),
    }

    #[async_trait]
    impl Evaluator<i32> for Filter {
        async fn evaluate(&self, input: Option<&i32>) -> Result<bool> {
            input
                .map(|&input| match self {
                    Filter::LessThan(value) => input < *value,
                })
                .ok_or_else(|| anyhow::anyhow!("No input found"))
        }
    }

    #[async_trait]
    impl Executable<i32> for MathOp {
        async fn execute(&self, input: Option<&i32>) -> Result<Option<i32>> {
            Ok(input.map(|&input| match self {
                MathOp::Add(value) => input + value,
                MathOp::Multiply(value) => input * value,
                MathOp::Identity => input,
            }))
        }
    }

    struct SumMergeable;

    #[async_trait]
    impl Mergeable<i32> for SumMergeable {
        async fn merge(&self, inputs: &[Option<&i32>]) -> Result<i32> {
            Ok(inputs.iter().flatten().copied().sum())
        }
    }

    #[tokio::test]
    async fn test_simple_pipeline() -> Result<()> {
        tests_common::init();

        let pipeline = Pipeline::<i32>::default()
            .add_node("Start", Node::Start)?
            .add_node("Double", Node::Task(Arc::new(MathOp::Multiply(2))))?
            .add_node("End", Node::End)?
            .connect("Start", "Double", Edge::default())?
            .connect("Double", "End", Edge::default())?
            .validate()?;
        let result = pipeline.run(Some(3)).await?;
        assert_eq!(result.len(), 1);
        let output_token = &result[0];
        assert_eq!(output_token.input, Some(6)); // 3 * 2 = 6
        Ok(())
    }
    #[tokio::test]
    async fn test_start_end_pipeline() -> Result<()> {
        tests_common::init();
        let pipeline = Pipeline::<i32>::default()
            .add_node("Start", Node::Start)?
            .add_node("End", Node::End)?
            .connect("Start", "End", Edge::default())?
            .validate()?;

        let result = pipeline.run(Some(3)).await?;
        assert_eq!(result.len(), 1);
        let output_token = &result[0];
        assert_eq!(output_token.input, Some(3));
        Ok(())
    }

    #[tokio::test]
    async fn test_pipeline_with_merge() -> Result<()> {
        tests_common::init();
        let pipeline = Pipeline::<i32>::default()
            .add_node("Start", Node::Start)?
            .add_node("Split", Node::Split)?
            .add_node("Task1", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task2", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task3", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task4", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task5", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task6", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task7", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task8", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task9", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Task10", Node::Task(Arc::new(MathOp::Identity)))?
            .add_node("Merge", Node::Merge(Arc::new(SumMergeable)))?
            .add_node("End", Node::End)?
            .connect("Start", "Split", Edge::default())?
            .connect("Split", "Task1", Edge::default())?
            .connect("Split", "Task2", Edge::default())?
            .connect("Split", "Task3", Edge::default())?
            .connect("Split", "Task4", Edge::default())?
            .connect("Split", "Task5", Edge::default())?
            .connect("Split", "Task6", Edge::default())?
            .connect("Split", "Task7", Edge::default())?
            .connect("Split", "Task8", Edge::default())?
            .connect("Split", "Task9", Edge::default())?
            .connect("Split", "Task10", Edge::default())?
            .connect("Task1", "Merge", Edge::default())?
            .connect("Task2", "Merge", Edge::default())?
            .connect("Task3", "Merge", Edge::default())?
            .connect("Task4", "Merge", Edge::default())?
            .connect("Task5", "Merge", Edge::default())?
            .connect("Task6", "Merge", Edge::default())?
            .connect("Task7", "Merge", Edge::default())?
            .connect("Task8", "Merge", Edge::default())?
            .connect("Task9", "Merge", Edge::default())?
            .connect("Task10", "Merge", Edge::default())?
            .connect("Merge", "End", Edge::default())?
            .validate()?;
        let result = pipeline.run(Some(3)).await?;
        assert_eq!(result.len(), 1);
        let output_token = &result[0];
        assert_eq!(output_token.input, Some(3 * 10));
        Ok(())
    }

    #[tokio::test]
    async fn test_pipeline_with_decision() -> Result<()> {
        tests_common::init();
        let answer = 10;
        let pipeline = Pipeline::<i32>::default()
            .add_node("Start", Node::Start)?
            .add_node("TaskLoopEntry", Node::Join)?
            .add_node("Task", Node::Task(Arc::new(MathOp::Add(1))))?
            .add_node("Decision", Node::Decision)?
            .add_node("End", Node::End)?
            .connect("Start", "TaskLoopEntry", Edge::default())?
            .connect("TaskLoopEntry", "Task", Edge::default())?
            .connect("Task", "Decision", Edge::default())?
            .connect(
                "Decision",
                "TaskLoopEntry",
                Edge::default().condition(ConditionType::Eval(Arc::new(Filter::LessThan(answer)))),
            )?
            .connect("Decision", "End", Edge::default())?
            .validate()?;
        let result = pipeline.run(Some(1)).await?;
        assert_eq!(result.len(), 1);
        let output_token = &result[0];
        assert_eq!(output_token.input, Some(answer));
        Ok(())
    }
}
