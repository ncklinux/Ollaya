use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize)]
struct DecideRequest<'a> {
    model: &'a str,
    state: &'a str,
    preset: &'a str,
    stream: bool,
}

// Ollaya returns polymorphic answer objects (choice, score, or noul)
#[derive(Deserialize, Debug, Default)]
struct AnswerItem {
    choice: Option<String>,
    score: Option<f64>,
    noul: Option<f64>,
}

#[derive(Deserialize, Debug, Default)]
struct Answers {
    intent: Option<AnswerItem>,
    is_urgent: Option<AnswerItem>,
    frustration: Option<AnswerItem>,
    refund_requested: Option<AnswerItem>,
    churn_risk: Option<AnswerItem>,
}

#[derive(Deserialize, Debug)]
struct DecideResponse {
    #[serde(default)]
    answers: Answers,
    #[serde(default)]
    eval_duration: u64,
}

// Normalized parsed decision struct
#[derive(Debug)]
pub struct Decision {
    pub intent: String,
    pub is_urgent: f64,
    pub frustration: f64,
    pub refund_requested: f64,
    pub churn_risk: f64,
    pub eval_duration_ms: f64,
}

pub struct OllayaClient {
    base_url: String,
    client: Client,
}

impl OllayaClient {
    pub fn new(base_url: Option<&str>) -> Self {
        Self {
            base_url: base_url
                .unwrap_or("http://192.168.56.106:11435")
                .to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to initialize HTTP client"),
        }
    }

    pub fn run(
        &self,
        model: &str,
        text: &str,
        preset: &str,
    ) -> Result<Decision, Box<dyn std::error::Error>> {
        let url = format!("{}/api/decide", self.base_url);
        let payload = DecideRequest {
            model,
            state: text,
            preset,
            stream: false,
        };

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .send()?
            .error_for_status()?;

        let raw: DecideResponse = response.json()?;

        // Helper to extract choice string
        let intent = raw
            .answers
            .intent
            .and_then(|i| i.choice)
            .unwrap_or_else(|| "other".to_string());

        // Helpers to extract numbers safely
        let is_urgent = raw.answers.is_urgent.and_then(|a| a.noul).unwrap_or(0.0);
        let frustration = raw
            .answers
            .frustration
            .and_then(|a| a.score.or(a.noul))
            .unwrap_or(0.0);
        let refund_requested = raw
            .answers
            .refund_requested
            .and_then(|a| a.noul)
            .unwrap_or(0.0);
        let churn_risk = raw.answers.churn_risk.and_then(|a| a.noul).unwrap_or(0.0);

        Ok(Decision {
            intent,
            is_urgent,
            frustration,
            refund_requested,
            churn_risk,
            eval_duration_ms: (raw.eval_duration as f64) / 1_000_000.0,
        })
    }
}

pub fn route_ticket(decision: &Decision) {
    println!("\n--- Ticket Routing Report ---");
    println!("Primary Intent : {}", decision.intent);
    println!("Frustration    : {:.2} / 3.0", decision.frustration);
    println!("Routing Actions:");

    // 1. Action Escalation Routes (Urgency, Churn, Anger)
    // High risk customer threatening to leave -> Retention / Account Manager team
    if decision.churn_risk >= 0.20 {
        // Example: send_to_retention_team(ticket_id);
        // Example: send_slack_alert("#customer-retention", "Churn risk detected!");
        // Example: crm.add_tag(ticket_id, "churn-risk");
        println!("  • ROUTE -> Escalated to Retention & VIP Support");
    }

    // Time sensitive issue -> Fast track queue (SLA < 15 min)
    if decision.is_urgent >= 0.35 {
        // Example: set_ticket_priority("P1_CRITICAL");
        // Example: pagerduty.trigger_incident("Urgent ticket received");
        // Example: crm.set_sla(ticket_id, 15);
        println!("  • ROUTE -> Priority set to HIGH / P1");
    }

    // Angry customer -> Experienced senior agent
    if decision.frustration >= 1.80 {
        // Example: route_to_senior_agents();
        // Example: crm.assign_group(ticket_id, "senior-escalations");
        // Example: notify_supervisor(ticket_id, "Customer highly frustrated");
        println!("  • ROUTE -> Assigned to Senior Empathy Agent");
    }

    // 2. Departmental Routing (Based on 'intent')
    match decision.intent.as_str() {
        "refund" => {
            // Example: route_to_billing_department();
            // Example: trigger_refund_verification_workflow();
            // Example: stripe.lookup_recent_charges(customer_id);
            println!("  • QUEUE -> Billing & Finance Department");
        }
        _ if decision.refund_requested >= 0.50 => {
            println!("  • QUEUE -> Billing & Finance Department");
        }
        "technical_help" => {
            // Example: route_to_tech_support();
            // Example: jira.create_issue("ENG", "Bug");
            // Example: crm.assign_group(ticket_id, "tier-2-engineering");
            println!("  • QUEUE -> Tier 2 Technical Support");
        }
        "cancellation" => {
            // Example: route_to_account_management();
            // Example: hubspot.update_deal_stage(deal_id, "At Risk");
            // Example: trigger_retention_offer_email(customer_id);
            println!("  • QUEUE -> Cancellations Queue");
        }
        "billing_question" => {
            // Example: route_to_billing_inbox();
            // Example: crm.assign_group(ticket_id, "standard-billing");
            println!("  • QUEUE -> Standard Billing Queue");
        }
        _ => {
            // Fallback route
            // Example: route_to_general_inbox();
            // Example: crm.assign_group(ticket_id, "tier-1-general");
            println!("  • QUEUE -> General Tier 1 Support Queue");
        }
    }
}

fn main() {
    let client = OllayaClient::new(None);

    let sample_text = "Hi team, we have a major client presentation in 3 hours and \
                       our team dashboard is showing an unexpected 500 error when loading reports. \
                       Could someone please help us get this back up urgently? Thank you!";

    println!("Analyzing: \"{}\"", sample_text);

    match client.run("laya:en", sample_text, "triage") {
        Ok(decision) => route_ticket(&decision),
        Err(err) => eprintln!("Request failed: {}", err),
    }
}
