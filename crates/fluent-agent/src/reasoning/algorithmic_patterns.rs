//! Algorithmic problem-solving patterns for intelligent agent reasoning
//!
//! This module provides pattern recognition and guidance for algorithmic
//! problem types, helping the agent identify and apply appropriate
//! algorithms for puzzles, search problems, and optimization tasks.

use serde::{Deserialize, Serialize};

/// Categories of algorithmic problems the agent can recognize
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlgorithmCategory {
    /// Search and traversal problems (BFS, DFS, etc.)
    Search,
    /// Pathfinding problems (A*, Dijkstra, etc.)
    Pathfinding,
    /// Dynamic programming problems (memoization, tabulation)
    DynamicProgramming,
    /// Graph algorithms (shortest path, connectivity, etc.)
    Graph,
    /// Sorting and ordering problems
    Sorting,
    /// Optimization problems (greedy, linear programming)
    Optimization,
    /// Puzzle solving (constraint satisfaction, backtracking)
    Puzzle,
    /// String algorithms (pattern matching, parsing)
    String,
    /// Tree algorithms (traversal, manipulation)
    Tree,
    /// Mathematical/numerical algorithms
    Mathematical,
}

/// Specific algorithm patterns with implementation guidance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmPattern {
    /// The category this pattern belongs to
    pub category: AlgorithmCategory,
    /// Specific algorithm name (e.g., "BFS", "A*", "Memoization")
    pub name: String,
    /// Keywords that indicate this pattern applies
    pub keywords: Vec<String>,
    /// Problem characteristics that match this pattern
    pub characteristics: Vec<String>,
    /// Confidence score for this pattern match (0.0-1.0)
    pub confidence: f64,
    /// Guidance prompt template for implementing this algorithm
    pub guidance: AlgorithmGuidance,
}

/// Detailed guidance for implementing an algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmGuidance {
    /// High-level approach description
    pub approach: String,
    /// Key data structures to use
    pub data_structures: Vec<String>,
    /// Implementation steps
    pub steps: Vec<String>,
    /// Common pitfalls to avoid
    pub pitfalls: Vec<String>,
    /// Time complexity (e.g., "O(n)", "O(n log n)")
    pub time_complexity: String,
    /// Space complexity
    pub space_complexity: String,
    /// Example code snippet (pseudocode or Rust)
    pub example_snippet: Option<String>,
}

/// Result of pattern detection for a problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDetectionResult {
    /// Detected patterns sorted by confidence
    pub patterns: Vec<AlgorithmPattern>,
    /// Keywords found in the problem description
    pub matched_keywords: Vec<String>,
    /// Problem characteristics identified
    pub characteristics: Vec<String>,
    /// Overall confidence in the detection (0.0-1.0)
    pub overall_confidence: f64,
    /// Recommended approach based on patterns
    pub recommendation: String,
}

/// Pattern detector for algorithmic problems
#[derive(Debug, Clone)]
pub struct AlgorithmPatternDetector {
    patterns: Vec<AlgorithmPattern>,
}

impl Default for AlgorithmPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl AlgorithmPatternDetector {
    /// Create a new pattern detector with built-in patterns
    pub fn new() -> Self {
        Self {
            patterns: Self::build_patterns(),
        }
    }

    /// Detect algorithmic patterns in a problem description
    pub fn detect(&self, problem_description: &str) -> PatternDetectionResult {
        let lower = problem_description.to_lowercase();
        let mut matched_patterns = Vec::new();
        let mut all_keywords = Vec::new();
        let mut all_characteristics = Vec::new();

        for pattern in &self.patterns {
            let mut keyword_matches = 0;
            let mut matched_kw = Vec::new();

            for keyword in &pattern.keywords {
                if lower.contains(&keyword.to_lowercase()) {
                    keyword_matches += 1;
                    matched_kw.push(keyword.clone());
                }
            }

            let mut char_matches = 0;
            let mut matched_chars = Vec::new();

            for characteristic in &pattern.characteristics {
                if lower.contains(&characteristic.to_lowercase()) {
                    char_matches += 1;
                    matched_chars.push(characteristic.clone());
                }
            }

            // Calculate confidence based on matches
            let total_indicators = pattern.keywords.len() + pattern.characteristics.len();
            let total_matches = keyword_matches + char_matches;

            if total_matches > 0 && total_indicators > 0 {
                let base_confidence = total_matches as f64 / total_indicators as f64;
                let adjusted_confidence = (base_confidence * pattern.confidence).min(1.0);

                let mut matched_pattern = pattern.clone();
                matched_pattern.confidence = adjusted_confidence;
                matched_patterns.push(matched_pattern);

                all_keywords.extend(matched_kw);
                all_characteristics.extend(matched_chars);
            }
        }

        // Sort by confidence descending
        matched_patterns.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate keywords and characteristics
        all_keywords.sort();
        all_keywords.dedup();
        all_characteristics.sort();
        all_characteristics.dedup();

        // Calculate overall confidence
        let overall_confidence = matched_patterns
            .first()
            .map(|p| p.confidence)
            .unwrap_or(0.0);

        // Generate recommendation
        let recommendation = self.generate_recommendation(&matched_patterns);

        PatternDetectionResult {
            patterns: matched_patterns,
            matched_keywords: all_keywords,
            characteristics: all_characteristics,
            overall_confidence,
            recommendation,
        }
    }

    /// Generate a prompt augmentation for the detected patterns
    pub fn generate_prompt_augmentation(&self, detection: &PatternDetectionResult) -> String {
        if detection.patterns.is_empty() {
            return String::new();
        }

        let mut prompt = String::new();
        prompt.push_str("\n## Algorithmic Pattern Analysis\n\n");

        if let Some(primary) = detection.patterns.first() {
            prompt.push_str(&format!(
                "**Detected Pattern**: {} ({:?})\n",
                primary.name, primary.category
            ));
            prompt.push_str(&format!(
                "**Confidence**: {:.0}%\n\n",
                primary.confidence * 100.0
            ));

            prompt.push_str("### Recommended Approach\n");
            prompt.push_str(&primary.guidance.approach);
            prompt.push_str("\n\n");

            prompt.push_str("### Key Data Structures\n");
            for ds in &primary.guidance.data_structures {
                prompt.push_str(&format!("- {}\n", ds));
            }
            prompt.push('\n');

            prompt.push_str("### Implementation Steps\n");
            for (i, step) in primary.guidance.steps.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, step));
            }
            prompt.push('\n');

            if !primary.guidance.pitfalls.is_empty() {
                prompt.push_str("### Common Pitfalls to Avoid\n");
                for pitfall in &primary.guidance.pitfalls {
                    prompt.push_str(&format!("- ⚠️ {}\n", pitfall));
                }
                prompt.push('\n');
            }

            prompt.push_str(&format!(
                "**Complexity**: Time: {}, Space: {}\n",
                primary.guidance.time_complexity, primary.guidance.space_complexity
            ));

            if let Some(ref snippet) = primary.guidance.example_snippet {
                prompt.push_str("\n### Example Pattern\n```\n");
                prompt.push_str(snippet);
                prompt.push_str("\n```\n");
            }
        }

        // List alternative approaches if multiple patterns detected
        if detection.patterns.len() > 1 {
            prompt.push_str("\n### Alternative Approaches\n");
            for pattern in detection.patterns.iter().skip(1).take(2) {
                prompt.push_str(&format!(
                    "- **{}** ({:.0}% confidence): {}\n",
                    pattern.name,
                    pattern.confidence * 100.0,
                    pattern.guidance.approach.lines().next().unwrap_or("")
                ));
            }
        }

        prompt
    }

    /// Generate a recommendation string based on detected patterns
    fn generate_recommendation(&self, patterns: &[AlgorithmPattern]) -> String {
        if patterns.is_empty() {
            return "No specific algorithmic pattern detected. Consider analyzing the problem structure more carefully.".to_string();
        }

        let primary = &patterns[0];
        let mut rec = format!(
            "This appears to be a {:?} problem. Consider using {} approach. ",
            primary.category, primary.name
        );

        if patterns.len() > 1 {
            rec.push_str(&format!(
                "Alternative: {} ({:.0}% confidence).",
                patterns[1].name,
                patterns[1].confidence * 100.0
            ));
        }

        rec
    }

    /// Build the comprehensive set of algorithm patterns
    fn build_patterns() -> Vec<AlgorithmPattern> {
        vec![
            // BFS Pattern
            AlgorithmPattern {
                category: AlgorithmCategory::Search,
                name: "Breadth-First Search (BFS)".to_string(),
                keywords: vec![
                    "shortest path".to_string(),
                    "minimum steps".to_string(),
                    "level order".to_string(),
                    "layers".to_string(),
                    "breadth first".to_string(),
                    "bfs".to_string(),
                    "fewest moves".to_string(),
                    "nearest".to_string(),
                ],
                characteristics: vec![
                    "unweighted graph".to_string(),
                    "find all reachable".to_string(),
                    "minimum distance".to_string(),
                    "same cost edges".to_string(),
                ],
                confidence: 0.9,
                guidance: AlgorithmGuidance {
                    approach: "Use BFS to explore all states at the current depth before moving deeper. This guarantees finding the shortest path in unweighted graphs.".to_string(),
                    data_structures: vec![
                        "Queue (FIFO) for frontier".to_string(),
                        "HashSet for visited states".to_string(),
                        "HashMap for parent tracking (path reconstruction)".to_string(),
                    ],
                    steps: vec![
                        "Define the state representation (what uniquely identifies a configuration)".to_string(),
                        "Initialize queue with starting state, mark as visited".to_string(),
                        "While queue not empty: dequeue state, check if goal, enqueue unvisited neighbors".to_string(),
                        "Track distances/parents if path reconstruction needed".to_string(),
                        "Return when goal found or queue exhausted".to_string(),
                    ],
                    pitfalls: vec![
                        "Forgetting to mark states as visited before enqueueing (causes infinite loops)".to_string(),
                        "Using wrong state representation (missing or redundant information)".to_string(),
                        "Not handling the case where goal is unreachable".to_string(),
                    ],
                    time_complexity: "O(V + E) where V = vertices/states, E = edges/transitions".to_string(),
                    space_complexity: "O(V) for the queue and visited set".to_string(),
                    example_snippet: Some(r#"
use std::collections::{VecDeque, HashSet};

fn bfs(start: State, is_goal: impl Fn(&State) -> bool) -> Option<usize> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();

    queue.push_back((start.clone(), 0));
    visited.insert(start);

    while let Some((state, dist)) = queue.pop_front() {
        if is_goal(&state) {
            return Some(dist);
        }
        for next in state.neighbors() {
            if visited.insert(next.clone()) {
                queue.push_back((next, dist + 1));
            }
        }
    }
    None
}
"#.to_string()),
                },
            },
            // DFS Pattern
            AlgorithmPattern {
                category: AlgorithmCategory::Search,
                name: "Depth-First Search (DFS)".to_string(),
                keywords: vec![
                    "explore all".to_string(),
                    "traverse".to_string(),
                    "dfs".to_string(),
                    "depth first".to_string(),
                    "backtrack".to_string(),
                    "recursion".to_string(),
                    "path exists".to_string(),
                ],
                characteristics: vec![
                    "find any path".to_string(),
                    "cycle detection".to_string(),
                    "topological sort".to_string(),
                    "connected components".to_string(),
                ],
                confidence: 0.85,
                guidance: AlgorithmGuidance {
                    approach: "Use DFS to explore as deep as possible before backtracking. Good for finding any path, detecting cycles, or exhaustive search.".to_string(),
                    data_structures: vec![
                        "Stack (or recursion call stack)".to_string(),
                        "HashSet for visited states".to_string(),
                        "Optional path vector for tracking current path".to_string(),
                    ],
                    steps: vec![
                        "Define base cases (goal reached, invalid state)".to_string(),
                        "Mark current state as visited".to_string(),
                        "Recursively explore each neighbor".to_string(),
                        "Backtrack by unmarking if needed (for path finding)".to_string(),
                        "Return result when found or after exhausting options".to_string(),
                    ],
                    pitfalls: vec![
                        "Stack overflow on deep recursion (use iterative with explicit stack)".to_string(),
                        "Not properly backtracking visited marks in all-paths problems".to_string(),
                        "Infinite loops without proper cycle detection".to_string(),
                    ],
                    time_complexity: "O(V + E)".to_string(),
                    space_complexity: "O(V) for recursion stack and visited set".to_string(),
                    example_snippet: None,
                },
            },
            // A* Pathfinding
            AlgorithmPattern {
                category: AlgorithmCategory::Pathfinding,
                name: "A* Search".to_string(),
                keywords: vec![
                    "shortest path".to_string(),
                    "optimal path".to_string(),
                    "heuristic".to_string(),
                    "a star".to_string(),
                    "a*".to_string(),
                    "weighted graph".to_string(),
                    "navigation".to_string(),
                    "routing".to_string(),
                ],
                characteristics: vec![
                    "weighted edges".to_string(),
                    "need optimal solution".to_string(),
                    "can estimate distance to goal".to_string(),
                    "admissible heuristic".to_string(),
                ],
                confidence: 0.92,
                guidance: AlgorithmGuidance {
                    approach: "A* combines actual cost (g) with heuristic estimate (h) to prioritize exploration. Uses f(n) = g(n) + h(n) for optimal pathfinding with admissible heuristics.".to_string(),
                    data_structures: vec![
                        "Priority queue (min-heap) ordered by f-score".to_string(),
                        "HashMap for g-scores (actual cost from start)".to_string(),
                        "HashMap for parent tracking".to_string(),
                        "HashSet for closed set (fully explored nodes)".to_string(),
                    ],
                    steps: vec![
                        "Define heuristic function (must be admissible - never overestimate)".to_string(),
                        "Initialize open set with start node, g(start)=0, f(start)=h(start)".to_string(),
                        "Pop node with lowest f-score from open set".to_string(),
                        "If goal, reconstruct path. Otherwise, expand neighbors.".to_string(),
                        "For each neighbor: calculate tentative g, update if better path found".to_string(),
                        "Continue until goal found or open set empty".to_string(),
                    ],
                    pitfalls: vec![
                        "Non-admissible heuristic leads to suboptimal paths".to_string(),
                        "Inefficient heuristic causes excessive exploration".to_string(),
                        "Not updating node when better path found".to_string(),
                    ],
                    time_complexity: "O(E log V) with good heuristic, O(b^d) worst case".to_string(),
                    space_complexity: "O(V) for open/closed sets".to_string(),
                    example_snippet: Some(r#"
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Reverse;

fn a_star(start: Node, goal: Node, heuristic: impl Fn(&Node) -> u32) -> Option<Vec<Node>> {
    let mut open = BinaryHeap::new();
    let mut g_scores = HashMap::new();
    let mut parents = HashMap::new();

    g_scores.insert(start.clone(), 0);
    open.push(Reverse((heuristic(&start), start.clone())));

    while let Some(Reverse((_, current))) = open.pop() {
        if current == goal {
            return Some(reconstruct_path(&parents, current));
        }
        let current_g = g_scores[&current];
        for (neighbor, cost) in current.neighbors_with_cost() {
            let tentative_g = current_g + cost;
            if tentative_g < *g_scores.get(&neighbor).unwrap_or(&u32::MAX) {
                g_scores.insert(neighbor.clone(), tentative_g);
                parents.insert(neighbor.clone(), current.clone());
                open.push(Reverse((tentative_g + heuristic(&neighbor), neighbor)));
            }
        }
    }
    None
}
"#.to_string()),
                },
            },
            // Dynamic Programming - Memoization
            AlgorithmPattern {
                category: AlgorithmCategory::DynamicProgramming,
                name: "Dynamic Programming (Memoization)".to_string(),
                keywords: vec![
                    "optimal".to_string(),
                    "maximum".to_string(),
                    "minimum".to_string(),
                    "count ways".to_string(),
                    "fibonacci".to_string(),
                    "dp".to_string(),
                    "overlapping subproblems".to_string(),
                    "memoization".to_string(),
                    "cache".to_string(),
                ],
                characteristics: vec![
                    "optimal substructure".to_string(),
                    "overlapping subproblems".to_string(),
                    "recursive definition".to_string(),
                    "build from smaller solutions".to_string(),
                ],
                confidence: 0.88,
                guidance: AlgorithmGuidance {
                    approach: "Identify the recurrence relation, then cache solutions to subproblems. Top-down (memoization) starts from the main problem and caches as you go.".to_string(),
                    data_structures: vec![
                        "HashMap or Vec for memoization cache".to_string(),
                        "State tuple/struct as cache key".to_string(),
                    ],
                    steps: vec![
                        "Define the state: what parameters uniquely identify a subproblem?".to_string(),
                        "Write the recurrence relation: how does solution depend on smaller subproblems?".to_string(),
                        "Identify base cases".to_string(),
                        "Implement recursive solution with memoization".to_string(),
                        "Call with the original problem parameters".to_string(),
                    ],
                    pitfalls: vec![
                        "Missing state dimensions (leads to incorrect caching)".to_string(),
                        "Incorrect base cases".to_string(),
                        "Stack overflow on deep recursion (consider bottom-up instead)".to_string(),
                    ],
                    time_complexity: "O(number of unique states × cost per state)".to_string(),
                    space_complexity: "O(number of unique states)".to_string(),
                    example_snippet: Some(r#"
use std::collections::HashMap;

fn solve_dp(n: usize, memo: &mut HashMap<usize, i64>) -> i64 {
    if let Some(&cached) = memo.get(&n) {
        return cached;
    }

    // Base cases
    if n == 0 { return 1; }
    if n == 1 { return 1; }

    // Recurrence relation
    let result = solve_dp(n - 1, memo) + solve_dp(n - 2, memo);
    memo.insert(n, result);
    result
}
"#.to_string()),
                },
            },
            // Sliding Puzzle / State Space Search
            AlgorithmPattern {
                category: AlgorithmCategory::Puzzle,
                name: "State Space Search (Sliding Puzzles)".to_string(),
                keywords: vec![
                    "puzzle".to_string(),
                    "sliding".to_string(),
                    "tile".to_string(),
                    "huarong".to_string(),
                    "klotski".to_string(),
                    "fifteen puzzle".to_string(),
                    "8 puzzle".to_string(),
                    "configuration".to_string(),
                    "rearrange".to_string(),
                ],
                characteristics: vec![
                    "discrete states".to_string(),
                    "valid moves".to_string(),
                    "goal configuration".to_string(),
                    "state transitions".to_string(),
                ],
                confidence: 0.95,
                guidance: AlgorithmGuidance {
                    approach: "Model the puzzle as a state space search. Each configuration is a node, valid moves create edges. Use BFS for fewest moves or A* with Manhattan distance heuristic for efficiency.".to_string(),
                    data_structures: vec![
                        "State struct (grid/board representation)".to_string(),
                        "HashSet<State> for visited configurations".to_string(),
                        "Queue (BFS) or PriorityQueue (A*)".to_string(),
                        "Move history for solution reconstruction".to_string(),
                    ],
                    steps: vec![
                        "Design state representation (compact, hashable)".to_string(),
                        "Implement move generation (all valid state transitions)".to_string(),
                        "Define goal state check".to_string(),
                        "For A*: implement heuristic (Manhattan distance, misplaced tiles)".to_string(),
                        "Run BFS/A* from initial state to goal".to_string(),
                        "Track moves for solution output".to_string(),
                    ],
                    pitfalls: vec![
                        "Inefficient state representation (use arrays, not strings)".to_string(),
                        "Not canonicalizing symmetric states".to_string(),
                        "Forgetting to check solvability before searching".to_string(),
                        "Poor heuristic causing excessive exploration".to_string(),
                    ],
                    time_complexity: "O(b^d) where b=branching factor, d=solution depth. Heuristics reduce this significantly.".to_string(),
                    space_complexity: "O(b^d) for storing visited states".to_string(),
                    example_snippet: None,
                },
            },
            // Backtracking
            AlgorithmPattern {
                category: AlgorithmCategory::Puzzle,
                name: "Backtracking".to_string(),
                keywords: vec![
                    "sudoku".to_string(),
                    "n-queens".to_string(),
                    "permutation".to_string(),
                    "combination".to_string(),
                    "subset".to_string(),
                    "generate all".to_string(),
                    "constraint".to_string(),
                    "valid".to_string(),
                ],
                characteristics: vec![
                    "constraint satisfaction".to_string(),
                    "incremental building".to_string(),
                    "pruning invalid branches".to_string(),
                ],
                confidence: 0.87,
                guidance: AlgorithmGuidance {
                    approach: "Build solution incrementally, abandoning partial solutions ('backtracking') as soon as they violate constraints. Use constraint propagation to prune search space.".to_string(),
                    data_structures: vec![
                        "Partial solution state".to_string(),
                        "Constraint validation function".to_string(),
                        "Solution collector".to_string(),
                    ],
                    steps: vec![
                        "Define what constitutes a complete solution".to_string(),
                        "Define constraint validation (is_valid)".to_string(),
                        "Implement recursive explore: make choice, recurse, undo choice".to_string(),
                        "Prune early when constraints violated".to_string(),
                        "Collect/return solutions when complete".to_string(),
                    ],
                    pitfalls: vec![
                        "Not pruning early enough (checking constraints too late)".to_string(),
                        "Forgetting to undo state when backtracking".to_string(),
                        "Missing constraint checks leading to invalid solutions".to_string(),
                    ],
                    time_complexity: "Depends on problem; often exponential but pruning helps".to_string(),
                    space_complexity: "O(solution depth) for recursion stack".to_string(),
                    example_snippet: None,
                },
            },
            // Dijkstra's Algorithm
            AlgorithmPattern {
                category: AlgorithmCategory::Graph,
                name: "Dijkstra's Algorithm".to_string(),
                keywords: vec![
                    "shortest path".to_string(),
                    "weighted graph".to_string(),
                    "dijkstra".to_string(),
                    "single source".to_string(),
                    "non-negative weights".to_string(),
                ],
                characteristics: vec![
                    "positive edge weights".to_string(),
                    "single source shortest path".to_string(),
                    "all shortest paths from source".to_string(),
                ],
                confidence: 0.90,
                guidance: AlgorithmGuidance {
                    approach: "Dijkstra finds shortest paths from a source to all other nodes in a graph with non-negative edge weights. Uses a priority queue to always process the closest unvisited node.".to_string(),
                    data_structures: vec![
                        "Priority queue (min-heap) for frontier".to_string(),
                        "HashMap<Node, Distance> for shortest distances".to_string(),
                        "Optional HashMap<Node, Node> for path reconstruction".to_string(),
                    ],
                    steps: vec![
                        "Initialize distances: source=0, all others=infinity".to_string(),
                        "Add source to priority queue".to_string(),
                        "Pop minimum distance node".to_string(),
                        "Update distances to neighbors if shorter path found".to_string(),
                        "Repeat until queue empty or target found".to_string(),
                    ],
                    pitfalls: vec![
                        "Using with negative edge weights (use Bellman-Ford instead)".to_string(),
                        "Not using decrease-key or re-adding nodes".to_string(),
                        "Processing already-finalized nodes".to_string(),
                    ],
                    time_complexity: "O((V + E) log V) with binary heap".to_string(),
                    space_complexity: "O(V)".to_string(),
                    example_snippet: None,
                },
            },
            // Union-Find / Disjoint Set
            AlgorithmPattern {
                category: AlgorithmCategory::Graph,
                name: "Union-Find (Disjoint Set)".to_string(),
                keywords: vec![
                    "connected".to_string(),
                    "component".to_string(),
                    "union".to_string(),
                    "find".to_string(),
                    "disjoint".to_string(),
                    "merge".to_string(),
                    "kruskal".to_string(),
                    "mst".to_string(),
                ],
                characteristics: vec![
                    "connectivity queries".to_string(),
                    "dynamic connectivity".to_string(),
                    "equivalence classes".to_string(),
                ],
                confidence: 0.85,
                guidance: AlgorithmGuidance {
                    approach: "Union-Find efficiently tracks connected components. Supports near-constant time union and find operations with path compression and union by rank.".to_string(),
                    data_structures: vec![
                        "parent array: parent[i] = parent of node i".to_string(),
                        "rank array: rank[i] = tree depth estimate".to_string(),
                    ],
                    steps: vec![
                        "Initialize: each node is its own parent".to_string(),
                        "Find: follow parent pointers to root, apply path compression".to_string(),
                        "Union: connect roots of two trees, use union by rank".to_string(),
                        "Use find() to check if two nodes are connected".to_string(),
                    ],
                    pitfalls: vec![
                        "Forgetting path compression (performance degrades)".to_string(),
                        "Not using union by rank (unbalanced trees)".to_string(),
                    ],
                    time_complexity: "O(α(n)) per operation, nearly O(1)".to_string(),
                    space_complexity: "O(n)".to_string(),
                    example_snippet: Some(r#"
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect(), rank: vec![0; n] }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]); // Path compression
        }
        self.parent[x]
    }

    fn union(&mut self, x: usize, y: usize) {
        let (rx, ry) = (self.find(x), self.find(y));
        if rx != ry {
            match self.rank[rx].cmp(&self.rank[ry]) {
                std::cmp::Ordering::Less => self.parent[rx] = ry,
                std::cmp::Ordering::Greater => self.parent[ry] = rx,
                std::cmp::Ordering::Equal => {
                    self.parent[ry] = rx;
                    self.rank[rx] += 1;
                }
            }
        }
    }
}
"#.to_string()),
                },
            },
            // Greedy Algorithms
            AlgorithmPattern {
                category: AlgorithmCategory::Optimization,
                name: "Greedy Algorithm".to_string(),
                keywords: vec![
                    "greedy".to_string(),
                    "locally optimal".to_string(),
                    "activity selection".to_string(),
                    "interval scheduling".to_string(),
                    "coin change".to_string(),
                    "huffman".to_string(),
                ],
                characteristics: vec![
                    "local optimum leads to global".to_string(),
                    "greedy choice property".to_string(),
                    "no backtracking needed".to_string(),
                ],
                confidence: 0.80,
                guidance: AlgorithmGuidance {
                    approach: "Make locally optimal choices at each step. Works when greedy choice property holds: local optimum contributes to global optimum.".to_string(),
                    data_structures: vec![
                        "Often just arrays and sorting".to_string(),
                        "Sometimes priority queue".to_string(),
                    ],
                    steps: vec![
                        "Prove greedy choice property (or recognize problem pattern)".to_string(),
                        "Define ordering/criteria for making choices".to_string(),
                        "Sort input if needed".to_string(),
                        "Iterate and make locally optimal choice at each step".to_string(),
                    ],
                    pitfalls: vec![
                        "Applying greedy to problems without greedy choice property".to_string(),
                        "Using wrong greedy criteria".to_string(),
                    ],
                    time_complexity: "Often O(n log n) for sorting + O(n) for greedy pass".to_string(),
                    space_complexity: "Usually O(1) to O(n)".to_string(),
                    example_snippet: None,
                },
            },
            // Binary Search
            AlgorithmPattern {
                category: AlgorithmCategory::Search,
                name: "Binary Search".to_string(),
                keywords: vec![
                    "sorted".to_string(),
                    "binary search".to_string(),
                    "find position".to_string(),
                    "search space".to_string(),
                    "monotonic".to_string(),
                    "log n".to_string(),
                ],
                characteristics: vec![
                    "sorted or monotonic".to_string(),
                    "can eliminate half".to_string(),
                    "search on answer".to_string(),
                ],
                confidence: 0.88,
                guidance: AlgorithmGuidance {
                    approach: "Repeatedly divide search space in half. Works on sorted data or when there's a monotonic predicate. 'Binary search on the answer' technique is powerful.".to_string(),
                    data_structures: vec![
                        "Just indices (low, high, mid)".to_string(),
                        "Sorted array or searchable predicate".to_string(),
                    ],
                    steps: vec![
                        "Define search space boundaries (low, high)".to_string(),
                        "Define condition for choosing left vs right half".to_string(),
                        "Loop while low < high (or low <= high depending on variant)".to_string(),
                        "Calculate mid, check condition, update bounds".to_string(),
                        "Return result based on final state".to_string(),
                    ],
                    pitfalls: vec![
                        "Off-by-one errors in bounds".to_string(),
                        "Integer overflow in mid calculation: use low + (high - low) / 2".to_string(),
                        "Infinite loops from incorrect bound updates".to_string(),
                    ],
                    time_complexity: "O(log n)".to_string(),
                    space_complexity: "O(1)".to_string(),
                    example_snippet: None,
                },
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_bfs_pattern() {
        let detector = AlgorithmPatternDetector::new();
        // BFS keywords: "bfs", "breadth first", "shortest path", "minimum steps", "fewest moves", "nearest"
        let result = detector.detect(
            "Use BFS to find shortest path with minimum steps and fewest moves to nearest goal",
        );

        assert!(
            !result.patterns.is_empty(),
            "Should detect at least one pattern"
        );
        // Check that BFS-related pattern is in the results
        let has_bfs = result.patterns.iter().any(|p| {
            p.name.contains("BFS")
                || p.name.contains("Breadth")
                || p.category == AlgorithmCategory::Search
        });
        assert!(has_bfs, "Should detect a search/BFS pattern");
    }

    #[test]
    fn test_detect_sliding_puzzle_pattern() {
        let detector = AlgorithmPatternDetector::new();
        let result =
            detector.detect("Solve the Huarong Dao sliding puzzle to reach the goal configuration");

        assert!(!result.patterns.is_empty());
        assert!(result
            .patterns
            .iter()
            .any(|p| p.name.contains("Sliding") || p.name.contains("State Space")));
    }

    #[test]
    fn test_detect_dp_pattern() {
        let detector = AlgorithmPatternDetector::new();
        let result = detector.detect(
            "Find the maximum profit with overlapping subproblems using optimal substructure",
        );

        assert!(!result.patterns.is_empty());
        assert!(result
            .patterns
            .iter()
            .any(|p| p.category == AlgorithmCategory::DynamicProgramming));
    }

    #[test]
    fn test_detect_a_star_pattern() {
        let detector = AlgorithmPatternDetector::new();
        let result =
            detector.detect("Find the optimal path in a weighted graph using a heuristic estimate");

        assert!(!result.patterns.is_empty());
        assert!(result.patterns.iter().any(|p| p.name.contains("A*")));
    }

    #[test]
    fn test_generate_prompt_augmentation() {
        let detector = AlgorithmPatternDetector::new();
        let result = detector.detect("Solve the 8-puzzle with minimum moves");
        let prompt = detector.generate_prompt_augmentation(&result);

        assert!(!prompt.is_empty());
        assert!(prompt.contains("Recommended Approach"));
        assert!(prompt.contains("Implementation Steps"));
    }

    #[test]
    fn test_no_pattern_detected() {
        let detector = AlgorithmPatternDetector::new();
        let result = detector.detect("Write a hello world program");

        // Should have low confidence or empty
        assert!(result.overall_confidence < 0.5 || result.patterns.is_empty());
    }

    #[test]
    fn test_multiple_patterns_detected() {
        let detector = AlgorithmPatternDetector::new();
        let result = detector
            .detect("Find the shortest path using optimal search in a graph with weighted edges");

        // Should detect multiple relevant patterns
        assert!(result.patterns.len() >= 2);
    }
}
