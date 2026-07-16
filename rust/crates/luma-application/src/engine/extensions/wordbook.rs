use super::super::*;
use luma_protocol::{WordReviewWordDto, WordbookStatsDto};

impl Engine {
    pub(crate) async fn handle_refresh_wordbook_review_stats(&self) {
        let Some(repo) = &self.wordbook else {
            return;
        };
        let stats = repo.stats().unwrap_or_default();
        let _ = self
            .emit(Event::WordbookReviewStatsUpdated {
                stats: WordbookStatsDto {
                    due: stats.due,
                    new_count: stats.new_count,
                    wrong: stats.wrong,
                    goal: stats.goal,
                    reviewed_today: stats.reviewed_today,
                    remaining_goal: stats.remaining_goal,
                },
            })
            .await;
    }

    pub(crate) async fn handle_load_wordbook_review(&self, queue: String) {
        let Some(repo) = &self.wordbook else {
            let _ = self
                .emit(Event::WordbookReviewLoaded {
                    queue,
                    words: vec![],
                    stats: WordbookStatsDto {
                        due: 0,
                        new_count: 0,
                        wrong: 0,
                        goal: 0,
                        reviewed_today: 0,
                        remaining_goal: 0,
                    },
                })
                .await;
            return;
        };
        let stats = repo.stats().unwrap_or_default();
        let queue_available = match queue.as_str() {
            "new" => stats.new_count.max(0) as usize,
            "wrong" => stats.wrong.max(0) as usize,
            _ => stats.due.max(0) as usize,
        };
        let goal_batch = if stats.remaining_goal > 0 {
            stats.remaining_goal as usize
        } else {
            stats.goal.max(1) as usize
        };
        let limit = goal_batch.min(queue_available.max(1)).clamp(1, 500);
        let words_result = match queue.as_str() {
            "new" => repo.list_new(limit),
            "wrong" => repo.list_wrong(limit),
            _ => repo.list_due(limit),
        };
        let words: Vec<WordReviewWordDto> = match words_result {
            Ok(entries) => {
                let mut words = Vec::with_capacity(entries.len());
                let mut review_items = Vec::with_capacity(entries.len());
                for (index, word) in entries.into_iter().enumerate() {
                    // Review actions use the same `wb:<id>` result contract as
                    // normal Wordbook search. Register the loaded words before
                    // emitting the event so grading does not depend on a prior
                    // search having populated the session cache.
                    review_items.push(luma_domain::SearchItem {
                        id: luma_domain::ResultId::new(format!("wb:{}", word.id)),
                        module_id: luma_domain::ModuleId::new("luma.wordbook"),
                        title: word.term.clone(),
                        subtitle: Some(word.meaning.clone()),
                        kind: "word".into(),
                        score: 90.0 - index as f64 * 0.1,
                        primary_action: luma_domain::ActionDescriptor {
                            id: luma_domain::ActionId::new("known"),
                            label: "Known".into(),
                            risk: luma_domain::ActionRisk::Safe,
                            confirmation: false,
                        },
                        secondary_actions: vec![],
                        ui_intent: None,
                        action_payload: None,
                    });
                    words.push(WordReviewWordDto {
                        id: word.id,
                        term: word.term,
                        phonetic: word.phonetic,
                        meaning: word.meaning,
                        example: word.example,
                    });
                }
                let evicted = {
                    let mut g = self.inner.lock().await;
                    g.insert_results_batch(
                        review_items
                            .into_iter()
                            .map(|item| (item.id.as_str().to_string(), item)),
                    )
                };
                if !evicted.is_empty() {
                    let _ = self
                        .emit(Event::ResultsChunk {
                            request_id: String::new(),
                            sequence: 0,
                            upserts: vec![],
                            removed_ids: evicted,
                        })
                        .await;
                }
                words
            }
            Err(_) => vec![],
        };
        let _ = self
            .emit(Event::WordbookReviewLoaded {
                queue,
                words,
                stats: WordbookStatsDto {
                    due: stats.due,
                    new_count: stats.new_count,
                    wrong: stats.wrong,
                    goal: stats.goal,
                    reviewed_today: stats.reviewed_today,
                    remaining_goal: stats.remaining_goal,
                },
            })
            .await;
    }
}
