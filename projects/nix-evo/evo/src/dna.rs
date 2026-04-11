use axum::{extract::Query, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;

// ─── Gene: a single configurable option in NixOS ─────────────────────────

/// A "gene" is one configurable option in a NixOS configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gene {
    pub name: String,
    pub value: GeneValue,
    pub category: GeneCategory,
    pub mutable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum GeneValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<String>),
}

impl GeneValue {
    pub fn as_string(&self) -> String {
        match self {
            GeneValue::Bool(b) => b.to_string(),
            GeneValue::Int(i) => i.to_string(),
            GeneValue::Float(f) => f.to_string(),
            GeneValue::String(s) => s.clone(),
            GeneValue::List(l) => format!("[{}]", l.join(", ")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GeneCategory {
    Security,
    Performance,
    Services,
    Network,
    Storage,
    Boot,
    Kernel,
    Other,
}

// ─── Genome: a complete configuration as DNA ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Genome {
    pub id: String,
    pub generation: u64,
    pub genes: Vec<Gene>,
    pub fitness: Option<FitnessScore>,
    pub parent_id: Option<String>,
    pub created_at: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessScore {
    pub build_speed: f64,
    pub disk_size: f64,
    pub security: f64,
    pub boot_time: f64,
    pub composite: f64,
}

impl FitnessScore {
    fn compute(genes: &[Gene]) -> Self {
        let mut security = 0.5_f64;
        let mut performance = 0.5_f64;
        let mut boot = 0.5_f64;
        let mut disk = 0.5_f64;

        for gene in genes {
            match (&gene.category, &gene.value) {
                (GeneCategory::Security, GeneValue::Bool(true)) => security += 0.05,
                (GeneCategory::Security, GeneValue::Bool(false)) => security -= 0.05,
                (GeneCategory::Services, GeneValue::Bool(true)) => {
                    performance -= 0.02;
                    boot -= 0.01;
                    disk -= 0.01;
                }
                (GeneCategory::Services, GeneValue::Bool(false)) => {
                    performance += 0.01;
                    boot += 0.01;
                    disk += 0.01;
                }
                (GeneCategory::Boot, GeneValue::Bool(true)) => boot += 0.03,
                (GeneCategory::Kernel, GeneValue::String(s)) if s.contains("fast") => boot += 0.04,
                (GeneCategory::Storage, GeneValue::String(s)) if s.contains("compress") => disk += 0.05,
                _ => {}
            }
        }

        let build_speed = performance.clamp(0.0, 1.0);
        let disk_size = disk.clamp(0.0, 1.0);
        let security = security.clamp(0.0, 1.0);
        let boot_time = boot.clamp(0.0, 1.0);

        let composite = build_speed * 0.2 + disk_size * 0.15 + security * 0.4 + boot_time * 0.25;

        FitnessScore { build_speed, disk_size, security, boot_time, composite }
    }
}

// ─── Evolution Engine ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    pub population_size: usize,
    pub generations: u64,
    pub mutation_rate: f64,
    pub crossover_rate: f64,
    pub elite_count: usize,
    pub weights: FitnessWeights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessWeights {
    pub build_speed: f64,
    pub disk_size: f64,
    pub security: f64,
    pub boot_time: f64,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            population_size: 20,
            generations: 10,
            mutation_rate: 0.15,
            crossover_rate: 0.7,
            elite_count: 4,
            weights: FitnessWeights {
                build_speed: 0.2,
                disk_size: 0.15,
                security: 0.4,
                boot_time: 0.25,
            },
        }
    }
}

pub struct DnaEngine {
    population: RwLock<Vec<Genome>>,
    history: RwLock<Vec<Genome>>,
    config: RwLock<EvolutionConfig>,
    next_id: std::sync::atomic::AtomicU64,
}

impl DnaEngine {
    pub fn new() -> Self {
        Self {
            population: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::new()),
            config: RwLock::new(EvolutionConfig::default()),
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> String {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        format!("genome-{}", id)
    }

    pub async fn seed(&self, base_genes: Vec<Gene>, count: usize) -> Vec<Genome> {
        let mut population = self.population.write().await;
        let now = chrono::Utc::now().to_rfc3339();

        let base = Genome {
            id: self.next_id(),
            generation: 0,
            genes: base_genes.clone(),
            fitness: Some(FitnessScore::compute(&base_genes)),
            parent_id: None,
            created_at: now.clone(),
            label: Some("base".to_string()),
        };
        population.push(base);

        for i in 1..count {
            let mutated = Self::mutate_genes(&base_genes, 0.2);
            let genome = Genome {
                id: self.next_id(),
                generation: 0,
                fitness: Some(FitnessScore::compute(&mutated)),
                genes: mutated,
                parent_id: Some(population[0].id.clone()),
                created_at: now.clone(),
                label: Some(format!("seed-{}", i)),
            };
            population.push(genome);
        }

        population.clone()
    }

    fn mutate_genes(genes: &[Gene], rate: f64) -> Vec<Gene> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut result = genes.to_vec();
        let mut hasher = DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        let seed = hasher.finish();

        for (i, gene) in result.iter_mut().enumerate() {
            if !gene.mutable { continue; }
            let chance = ((seed.wrapping_mul(i as u64 + 1)) % 1000) as f64 / 1000.0;
            if chance < rate {
                gene.value = Self::mutate_value(&gene.value, seed.wrapping_add(i as u64));
            }
        }
        result
    }

    fn mutate_value(val: &GeneValue, seed: u64) -> GeneValue {
        match val {
            GeneValue::Bool(b) => GeneValue::Bool(!b),
            GeneValue::Int(i) => {
                let delta = (seed % 10) as i64 - 5;
                GeneValue::Int(i + delta)
            }
            GeneValue::Float(f) => {
                let delta = (seed % 100) as f64 / 100.0 - 0.5;
                GeneValue::Float((f + delta).max(0.0).min(1.0))
            }
            GeneValue::String(_) => {
                let variants = ["enabled", "disabled", "auto", "strict", "permissive", "fast", "balanced", "compress"];
                let idx = (seed as usize) % variants.len();
                GeneValue::String(variants[idx].to_string())
            }
            GeneValue::List(l) => {
                let mut new_l = l.clone();
                if !new_l.is_empty() && seed % 3 == 0 {
                    new_l.pop();
                } else {
                    new_l.push(format!("variant-{}", seed % 100));
                }
                GeneValue::List(new_l)
            }
        }
    }

    fn crossover(parent_a: &[Gene], parent_b: &[Gene], rate: f64, seed: u64) -> Vec<Gene> {
        let mut child = parent_a.to_vec();
        for (i, gene_b) in parent_b.iter().enumerate() {
            let chance = ((seed.wrapping_mul(i as u64 + 7)) % 1000) as f64 / 1000.0;
            if chance < rate && i < child.len() {
                child[i].value = gene_b.value.clone();
            }
        }
        child
    }

    pub async fn evolve(&self, evo_config: Option<EvolutionConfig>) -> EvolutionResult {
        let config = evo_config.unwrap_or_else(|| self.config.read().await.clone());
        let mut population = self.population.write().await;

        if population.is_empty() {
            return EvolutionResult {
                status: "empty_population".to_string(),
                best_genome: None,
                generations_completed: 0,
                population_summary: vec![],
                fitness_history: vec![],
            };
        }

        let mut fitness_history: Vec<FitnessGeneration> = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        for gen in 0..config.generations {
            for genome in population.iter_mut() {
                genome.fitness = Some(FitnessScore::compute(&genome.genes));
            }

            population.sort_by(|a, b| {
                let fa = a.fitness.as_ref().map(|f| f.composite).unwrap_or(0.0);
                let fb = b.fitness.as_ref().map(|f| f.composite).unwrap_or(0.0);
                fb.partial_cmp(&fa).unwrap_or(std::cmp::Ordering::Equal)
            });

            let best = population.first().and_then(|g| g.fitness.as_ref()).cloned();
            let avg = if !population.is_empty() {
                let sum: f64 = population.iter()
                    .filter_map(|g| g.fitness.as_ref().map(|f| f.composite))
                    .sum();
                sum / population.len() as f64
            } else { 0.0 };
            fitness_history.push(FitnessGeneration {
                generation: gen,
                best_fitness: best,
                avg_fitness: avg,
                population_size: population.len(),
            });

            if gen < config.generations - 1 {
                let elite: Vec<Genome> = population[..config.elite_count.min(population.len())].to_vec();
                let mut new_pop = elite.clone();

                while new_pop.len() < config.population_size {
                    let p1_idx = (gen as usize) % elite.len();
                    let p2_idx = (gen as usize + 1) % elite.len();
                    let child_genes = Self::crossover(
                        &elite[p1_idx].genes, &elite[p2_idx].genes,
                        config.crossover_rate, gen.wrapping_mul(31),
                    );
                    let mutated = Self::mutate_genes(&child_genes, config.mutation_rate);

                    new_pop.push(Genome {
                        id: self.next_id(),
                        generation: gen + 1,
                        genes: mutated,
                        fitness: None,
                        parent_id: Some(elite[p1_idx].id.clone()),
                        created_at: now.clone(),
                        label: Some(format!("evo-{}-{}", gen, new_pop.len())),
                    });
                }
                *population = new_pop;
            }
        }

        let mut history = self.history.write().await;
        history.extend(population.clone());

        let best_genome = population.first().cloned();
        let summary: Vec<GenomeSummary> = population.iter().take(10).map(|g| GenomeSummary {
            id: g.id.clone(),
            label: g.label.clone(),
            fitness: g.fitness.clone(),
            gene_count: g.genes.len(),
        }).collect();

        EvolutionResult {
            status: "completed".to_string(),
            best_genome,
            generations_completed: config.generations,
            population_summary: summary,
            fitness_history,
        }
    }

    pub async fn get_population(&self) -> Vec<Genome> {
        self.population.read().await.clone()
    }

    pub async fn get_history(&self) -> Vec<Genome> {
        self.history.read().await.clone()
    }
}

// ─── API types ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EvolutionResult {
    pub status: String,
    pub best_genome: Option<Genome>,
    pub generations_completed: u64,
    pub population_summary: Vec<GenomeSummary>,
    pub fitness_history: Vec<FitnessGeneration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GenomeSummary {
    pub id: String,
    pub label: Option<String>,
    pub fitness: Option<FitnessScore>,
    pub gene_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FitnessGeneration {
    pub generation: u64,
    pub best_fitness: Option<FitnessScore>,
    pub avg_fitness: f64,
    pub population_size: usize,
}

#[derive(Debug, Deserialize)]
pub struct EvolveRequest {
    pub genes: Option<Vec<Gene>>,
    pub config: Option<EvolutionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PopulationQuery {
    pub limit: Option<usize>,
}

// ─── Global engine ───────────────────────────────────────────────────────

use std::sync::LazyLock;
static DNA_ENGINE: LazyLock<Arc<DnaEngine>> = LazyLock::new(|| Arc::new(DnaEngine::new()));

/// POST /api/dna/evolve
pub async fn handle_evolve(Json(req): Json<EvolveRequest>) -> Result<impl IntoResponse, AppError> {
    let engine = DNA_ENGINE.clone();
    if let Some(genes) = req.genes {
        let count = req.config.as_ref().map(|c| c.population_size).unwrap_or(20);
        engine.seed(genes, count).await;
    }
    let result = engine.evolve(req.config).await;
    Ok(Json(result))
}

/// GET /api/dna/population
pub async fn handle_population(Query(q): Query<PopulationQuery>) -> impl IntoResponse {
    let engine = DNA_ENGINE.clone();
    let mut pop = engine.get_population().await;
    if let Some(limit) = q.limit { pop.truncate(limit); }
    let summaries: Vec<GenomeSummary> = pop.iter().map(|g| GenomeSummary {
        id: g.id.clone(), label: g.label.clone(),
        fitness: g.fitness.clone(), gene_count: g.genes.len(),
    }).collect();
    Json(serde_json::json!({ "population_size": pop.len(), "genomes": summaries }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fitness_computation() {
        let genes = vec![
            Gene { name: "services.nginx.enable".into(), value: GeneValue::Bool(true), category: GeneCategory::Services, mutable: true },
            Gene { name: "security.sudo.enable".into(), value: GeneValue::Bool(true), category: GeneCategory::Security, mutable: true },
        ];
        let fitness = FitnessScore::compute(&genes);
        assert!(fitness.composite > 0.0);
        assert!(fitness.security > 0.5);
    }

    #[test]
    fn test_mutate_bool() {
        let val = GeneValue::Bool(true);
        let mutated = DnaEngine::mutate_value(&val, 42);
        assert!(matches!(mutated, GeneValue::Bool(false)));
    }

    #[tokio::test]
    async fn test_seed_and_evolve() {
        let engine = DnaEngine::new();
        let genes = vec![
            Gene { name: "services.nginx.enable".into(), value: GeneValue::Bool(true), category: GeneCategory::Services, mutable: true },
            Gene { name: "security.firewall.enable".into(), value: GeneValue::Bool(true), category: GeneCategory::Security, mutable: true },
        ];
        engine.seed(genes, 10).await;
        let config = EvolutionConfig { population_size: 10, generations: 3, mutation_rate: 0.2, crossover_rate: 0.7, elite_count: 2, ..Default::default() };
        let result = engine.evolve(Some(config)).await;
        assert_eq!(result.status, "completed");
        assert_eq!(result.generations_completed, 3);
    }
}
