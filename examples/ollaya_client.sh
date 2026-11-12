#!/usr/bin/env bash
set -euo pipefail

# Force standard decimal format (dot instead of comma) regardless of system language
export LC_ALL=C

# Configuration
BASE_URL="http://192.168.56.106:11435"
MODEL="laya:en"
PRESET="triage"

# Helper function to check floating point comparisons (returns 0 for true, 1 for false)
compare_float() {
    awk -v val="$1" -v thresh="$2" 'BEGIN { exit !(val >= thresh) }'
}

# Routing function based on extracted decision values
route_ticket() {
    local intent="$1"
    local frustration="$2"
    local churn_risk="$3"
    local is_urgent="$4"
    local refund_requested="$5"

    echo ""
    echo "--- Ticket Routing Report ---"
    printf "Primary Intent : %s\n" "$intent"
    printf "Frustration    : %.2f / 3.0\n" "$frustration"
    echo "Routing Actions:"

    # =========================================================
    # 1. Action Escalation Routes (Urgency, Churn, Anger)
    # =========================================================

    # High risk customer threatening to leave -> Retention / Account Manager team
    if compare_float "$churn_risk" "0.20"; then
        # Example: ./scripts/send_to_retention.sh "$ticket_id"
        # Example: curl -X POST -H 'Content-type: application/json' --data '{"text":"Churn risk detected!"}' "$SLACK_WEBHOOK_URL"
        echo "  • ROUTE -> Escalated to Retention & VIP Support"
    fi

    # Time sensitive issue -> Fast track queue (SLA < 15 min)
    if compare_float "$is_urgent" "0.35"; then
        # Example: ./scripts/set_ticket_priority.sh "P1_CRITICAL"
        # Example: pd-trigger-incident --summary="Urgent ticket received"
        echo "  • ROUTE -> Priority set to HIGH / P1"
    fi

    # Angry customer -> Experienced senior agent
    if compare_float "$frustration" "1.80"; then
        # Example: ./scripts/assign_agent_group.sh "senior-escalations"
        echo "  • ROUTE -> Assigned to Senior Empathy Agent"
    fi

    # =========================================================
    # 2. Departmental Routing (Based on 'intent')
    # =========================================================
    if [ "$intent" = "refund" ] || compare_float "$refund_requested" "0.50"; then
        # Example: ./scripts/route_to_department.sh "billing"
        # Example: ./scripts/stripe_lookup.sh "$customer_id"
        echo "  • QUEUE -> Billing & Finance Department"
    elif [ "$intent" = "technical_help" ]; then
        # Example: ./scripts/create_jira_issue.sh --project="ENG" --type="Bug"
        echo "  • QUEUE -> Tier 2 Technical Support"
    elif [ "$intent" = "cancellation" ]; then
        # Example: ./scripts/update_hubspot_deal.sh "At Risk"
        echo "  • QUEUE -> Cancellations Queue"
    elif [ "$intent" = "billing_question" ]; then
        # Example: ./scripts/route_to_department.sh "standard-billing"
        echo "  • QUEUE -> Standard Billing Queue"
    else
        # Fallback route
        # Example: ./scripts/route_to_department.sh "general-tier-1"
        echo "  • QUEUE -> General Tier 1 Support Queue"
    fi
}

# Example Usage
sample_text="Hi team, we have a major client presentation in 3 hours and our team dashboard is showing an unexpected 500 error when loading reports. Could someone please help us get this back up urgently? Thank you!"

printf 'Analyzing: "%s"\n' "$sample_text"

# Construct JSON payload using jq to prevent escaping issues
PAYLOAD=$(jq -n \
    --arg model "$MODEL" \
    --arg state "$sample_text" \
    --arg preset "$PRESET" \
    '{model: $model, state: $state, preset: $preset, stream: false}')

# Send POST request to Ollaya API
RESPONSE=$(curl -s --fail --max-time 30 -X POST "${BASE_URL}/api/decide" \
    -H "Content-Type: application/json" \
    -d "$PAYLOAD")

# Parse JSON values safely with default fallbacks
INTENT=$(echo "$RESPONSE" | jq -r '.answers.intent.choice // "other"')
FRUSTRATION=$(echo "$RESPONSE" | jq -r '.answers.frustration.score // 0.0')
CHURN_RISK=$(echo "$RESPONSE" | jq -r '.answers.churn_risk.noul // 0.0')
IS_URGENT=$(echo "$RESPONSE" | jq -r '.answers.is_urgent.noul // 0.0')
REFUND_REQUESTED=$(echo "$RESPONSE" | jq -r '.answers.refund_requested.noul // 0.0')

# Execute routing logic
route_ticket "$INTENT" "$FRUSTRATION" "$CHURN_RISK" "$IS_URGENT" "$REFUND_REQUESTED"