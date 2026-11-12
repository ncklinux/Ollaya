import requests


# Client for Ollaya, decision/classification API
class OllayaClient:
    def __init__(self, base_url="http://192.168.56.106:11435"):
        # Base URL of the Ollaya server
        self.base_url = base_url

    def run(self, model="laya:en", text="", preset="triage"):
        # Ollaya serves decisions on /api/decide
        url = f"{self.base_url}/api/decide"
        payload = {"model": model, "state": text, "preset": preset, "stream": False}
        try:
            # Send classification request
            response = requests.post(url, json=payload, timeout=30)
            response.raise_for_status()
            data = response.json()
            return self.parse_response(data)
        except requests.RequestException as e:
            print(f"Request failed: {e}")
            return None

    def parse_response(self, data):
        """
        Parses Ollaya's JSON response, keeping raw numeric values (floats)
        so downstream routing logic can apply custom thresholds
        """
        answers = data.get("answers", {})

        def extract_value(item):
            if not isinstance(item, dict):
                return item
            # Choice based questions (e.g. 'intent')
            if "choice" in item:
                return item["choice"]
            # Numeric score (e.g. 'frustration' 0.0 - 3.0)
            if "score" in item:
                return float(item["score"])
            # Binary probability (e.g. 'noul' 0.0 - 1.0)
            if "noul" in item:
                return float(item["noul"])
            return None

        return {
            "intent": extract_value(answers.get("intent")),
            "is_urgent": extract_value(answers.get("is_urgent")) or 0.0,
            "frustration": extract_value(answers.get("frustration")) or 0.0,
            "refund_requested": extract_value(answers.get("refund_requested")) or 0.0,
            "churn_risk": extract_value(answers.get("churn_risk")) or 0.0,
            "usage": data.get("usage", {}),
            "eval_duration_ms": data.get("eval_duration", 0) / 1_000_000,
        }


def route_ticket(decision):
    """
    Evaluates classification signals against calibrated business thresholds
    and executes departmental and escalation routing
    """
    if not decision:
        print("Routing error: No decision available.")
        return
    print("\n--- Ticket Routing Report ---")
    print(f"Primary Intent : {decision['intent']}")
    print(f"Frustration    : {decision['frustration']:.2f} / 3.0")
    print("Routing Actions:")

    # 1. Action escalation routes (Urgency, Churn, Anger)
    # High risk customer threatening to leave -> Retention / Account Manager team
    if decision["churn_risk"] >= 0.20:
        # Example: send_to_retention_team(ticket_id)
        # Example: send_slack_alert(channel="#customer-retention", text="Churn risk detected!")
        # Example: crm.add_tag(ticket_id, "churn-risk")
        print("  • ROUTE -> Escalated to Retention & VIP Support")
    # Time sensitive issue -> Fast track queue (SLA < 15 min)
    if decision["is_urgent"] >= 0.35:
        # Example: set_ticket_priority("P1_CRITICAL")
        # Example: pagerduty.trigger_incident(summary="Urgent ticket received")
        # Example: crm.set_sla(ticket_id, minutes=15)
        print("  • ROUTE -> Priority set to HIGH / P1")
    # Angry customer -> Experienced senior agent
    if decision["frustration"] >= 1.8:
        # Example: route_to_senior_agents()
        # Example: crm.assign_group(ticket_id, "senior-escalations")
        # Example: notify_supervisor(ticket_id, reason="Customer highly frustrated")
        print("  • ROUTE -> Assigned to Senior Empathy Agent")
    # 2. Departmental routing (based on 'intent')
    intent = decision["intent"]
    if intent == "refund" or decision["refund_requested"] >= 0.50:
        # Example: route_to_billing_department()
        # Example: trigger_refund_verification_workflow()
        # Example: stripe.lookup_recent_charges(customer_id)
        print("  • QUEUE -> Billing & Finance Department")
    elif intent == "technical_help":
        # Example: route_to_tech_support()
        # Example: jira.create_issue(project="ENG", issue_type="Bug")
        # Example: crm.assign_group(ticket_id, "tier-2-engineering")
        print("  • QUEUE -> Tier 2 Technical Support")
    elif intent == "cancellation":
        # Example: route_to_account_management()
        # Example: hubspot.update_deal_stage(deal_id, "At Risk")
        # Example: trigger_retention_offer_email(customer_id)
        print("  • QUEUE -> Cancellations Queue")
    elif intent == "billing_question":
        # Example: route_to_billing_inbox()
        # Example: crm.assign_group(ticket_id, "standard-billing")
        print("  • QUEUE -> Standard Billing Queue")
    else:
        # Fallback route
        # Example: route_to_general_inbox()
        # Example: crm.assign_group(ticket_id, "tier-1-general")
        print("  • QUEUE -> General Tier 1 Support Queue")


# Example usage
if __name__ == "__main__":
    client = OllayaClient()

    sample_text = (
        "Hi team, we have a major client presentation in 3 hours and "
        "our team dashboard is showing an unexpected 500 error when loading reports. "
        "Could someone please help us get this back up urgently? Thank you!"
    )

    print(f'Analyzing: "{sample_text}"')
    result = client.run(text=sample_text)
    route_ticket(result)
