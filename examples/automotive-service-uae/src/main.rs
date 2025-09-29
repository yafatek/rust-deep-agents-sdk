use agents_sdk::{
    agent::AgentHandle,
    llm::StreamChunk,
    state::AgentStateSnapshot,
    tool, ConfigurableAgentBuilder, OpenAiChatModel, OpenAiConfig, SubAgentConfig,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct DiagnosticResult {
    issue_type: String,
    severity: String,
    recommended_service: String,
    estimated_cost_aed: f64,
    estimated_duration_hours: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct SupportTicket {
    ticket_id: String,
    customer_name: String,
    vehicle_model: String,
    issue_description: String,
    priority: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Appointment {
    appointment_id: String,
    customer_name: String,
    vehicle_model: String,
    service_type: String,
    date: String,
    time: String,
    location: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct PaymentLink {
    payment_id: String,
    amount_aed: f64,
    description: String,
    link: String,
    expires_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Notification {
    notification_id: String,
    channel: String, // SMS, Email, WhatsApp
    recipient: String,
    message: String,
    sent_at: String,
}

// ============================================================================
// Diagnostic Tools
// ============================================================================

#[tool("Diagnoses car issues based on symptoms and provides recommendations")]
fn diagnose_car_issue(
    symptoms: String,
    vehicle_make: String,
    vehicle_model: String,
    year: i32,
    mileage_km: i32,
) -> String {
    // Simulate AI-powered diagnosis
    let diagnostic = DiagnosticResult {
        issue_type: format!("Based on '{}', likely issue detected", symptoms),
        severity: if symptoms.to_lowercase().contains("noise") {
            "Medium".to_string()
        } else if symptoms.to_lowercase().contains("smoke") {
            "High".to_string()
        } else {
            "Low".to_string()
        },
        recommended_service: format!(
            "Recommended: Full inspection + {} service for {} {} ({})",
            if mileage_km > 100000 {
                "major"
            } else {
                "minor"
            },
            vehicle_make,
            vehicle_model,
            year
        ),
        estimated_cost_aed: if symptoms.to_lowercase().contains("engine") {
            2500.0
        } else if symptoms.to_lowercase().contains("brake") {
            800.0
        } else {
            500.0
        },
        estimated_duration_hours: 2.5,
    };

    serde_json::to_string_pretty(&diagnostic).unwrap_or_else(|_| format!("{:?}", diagnostic))
}

#[tool("Calculates service cost based on vehicle and service type")]
fn calculate_service_cost(
    vehicle_make: String,
    vehicle_model: String,
    service_type: String,
    year: i32,
) -> String {
    let base_cost = match service_type.to_lowercase().as_str() {
        s if s.contains("oil") => 250.0,
        s if s.contains("brake") => 800.0,
        s if s.contains("tire") => 600.0,
        s if s.contains("engine") => 2000.0,
        s if s.contains("ac") => 450.0,
        s if s.contains("inspection") => 150.0,
        _ => 500.0,
    };

    // Premium brands cost more
    let brand_multiplier = match vehicle_make.to_lowercase().as_str() {
        "bmw" | "mercedes" | "audi" | "porsche" => 1.5,
        "lexus" | "infiniti" | "cadillac" => 1.3,
        "toyota" | "nissan" | "honda" => 1.0,
        _ => 1.1,
    };

    // Older cars may need more work
    let age = 2025 - year;
    let age_factor = if age > 10 { 1.2 } else { 1.0 };

    let final_cost = base_cost * brand_multiplier * age_factor;

    format!(
        "Estimated Cost Breakdown:\n\
         - Base service ({}): {:.2} AED\n\
         - Vehicle brand factor ({}): x{:.2}\n\
         - Vehicle age factor ({} years): x{:.2}\n\
         - Total Estimate: {:.2} AED\n\
         - Note: Price includes VAT (5%)",
        service_type, base_cost, vehicle_make, brand_multiplier, age, age_factor, final_cost
    )
}

// ============================================================================
// Ticketing Tools
// ============================================================================

#[tool("Creates a support ticket for the customer issue")]
fn create_support_ticket(
    customer_name: String,
    vehicle_model: String,
    issue_description: String,
    priority: String,
) -> String {
    let ticket = SupportTicket {
        ticket_id: format!("TKT-UAE-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        customer_name,
        vehicle_model,
        issue_description,
        priority,
        status: "OPEN".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    format!(
        "✅ Support Ticket Created Successfully!\n\n{}",
        serde_json::to_string_pretty(&ticket).unwrap_or_else(|_| format!("{:?}", ticket))
    )
}

#[tool("Checks the status of a support ticket")]
fn check_ticket_status(ticket_id: String) -> String {
    // Simulate ticket status check
    format!(
        "Ticket Status for {}:\n\
         - Status: IN_PROGRESS\n\
         - Assigned to: Service Center - Dubai Branch\n\
         - Last Updated: {} \n\
         - Expected Completion: Within 24 hours\n\
         - Next Action: Vehicle inspection scheduled",
        ticket_id,
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    )
}

// ============================================================================
// Booking Tools
// ============================================================================

#[tool("Checks availability for service appointments")]
fn check_availability(date: String, service_center: String) -> String {
    // Simulate availability check
    format!(
        "📅 Availability for {} at {}:\n\n\
         Available Time Slots:\n\
         - 08:00 AM - 10:00 AM ✅\n\
         - 10:00 AM - 12:00 PM ✅\n\
         - 02:00 PM - 04:00 PM ✅\n\
         - 04:00 PM - 06:00 PM ❌ (Fully Booked)\n\n\
         Note: We recommend booking morning slots for faster service.",
        date, service_center
    )
}

#[tool("Books a service appointment for the customer")]
fn book_appointment(
    customer_name: String,
    vehicle_model: String,
    service_type: String,
    date: String,
    time: String,
    service_center: String,
) -> String {
    let appointment = Appointment {
        appointment_id: format!("APT-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        customer_name,
        vehicle_model,
        service_type,
        date,
        time,
        location: service_center,
    };

    format!(
        "✅ Appointment Booked Successfully!\n\n{}\n\n\
         📍 Location: Al Quoz Industrial Area, Dubai, UAE\n\
         📞 Contact: +971-4-XXX-XXXX\n\
         ⏰ Please arrive 10 minutes early\n\
         🚗 Free pickup/drop service available",
        serde_json::to_string_pretty(&appointment).unwrap_or_else(|_| format!("{:?}", appointment))
    )
}

// ============================================================================
// Payment Tools
// ============================================================================

#[tool("Generates a secure payment link for the customer")]
fn generate_payment_link(
    customer_name: String,
    amount_aed: f64,
    description: String,
) -> String {
    let payment = PaymentLink {
        payment_id: format!("PAY-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        amount_aed,
        description: description.clone(),
        link: format!(
            "https://pay.autouae.ae/checkout/{}",
            Uuid::new_v4().to_string()
        ),
        expires_at: (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
    };

    format!(
        "💳 Payment Link Generated!\n\n{}\n\n\
         Payment Methods Accepted:\n\
         - Credit/Debit Cards (Visa, Mastercard)\n\
         - Apple Pay / Google Pay\n\
         - Bank Transfer (ENBD, ADCB, FAB)\n\
         - Cash on Delivery\n\n\
         Security: 256-bit SSL encryption\n\
         Link expires in 24 hours",
        serde_json::to_string_pretty(&payment).unwrap_or_else(|_| format!("{:?}", payment))
    )
}

#[tool("Processes payment confirmation")]
fn confirm_payment(payment_id: String, method: String) -> String {
    format!(
        "✅ Payment Confirmed!\n\n\
         Payment ID: {}\n\
         Method: {}\n\
         Status: SUCCESSFUL\n\
         Processed At: {}\n\
         Receipt: Will be sent to your email\n\n\
         Thank you for your payment!",
        payment_id,
        method,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    )
}

// ============================================================================
// Notification Tools
// ============================================================================

#[tool("Sends notification to customer via SMS, Email, or WhatsApp")]
fn send_notification(
    channel: String,
    recipient: String,
    message: String,
) -> String {
    let notification = Notification {
        notification_id: format!("NOT-{}", Uuid::new_v4().to_string()[..8].to_uppercase()),
        channel: channel.clone(),
        recipient: recipient.clone(),
        message: message.clone(),
        sent_at: chrono::Utc::now().to_rfc3339(),
    };

    let channel_info = match channel.to_lowercase().as_str() {
        "sms" => "📱 SMS sent via Etisalat/Du",
        "email" => "📧 Email sent via SendGrid",
        "whatsapp" => "💬 WhatsApp message sent",
        _ => "📬 Notification sent",
    };

    format!(
        "✅ Notification Sent!\n\n{}\n\n{}\nRecipient: {}\nDelivery Status: DELIVERED",
        serde_json::to_string_pretty(&notification).unwrap_or_else(|_| format!("{:?}", notification)),
        channel_info,
        recipient
    )
}

// ============================================================================
// Feedback Tools
// ============================================================================

#[tool("Collects customer feedback and satisfaction rating")]
fn collect_feedback(
    customer_name: String,
    service_type: String,
    rating: i32,
    comments: String,
) -> String {
    let emoji = match rating {
        5 => "⭐⭐⭐⭐⭐",
        4 => "⭐⭐⭐⭐",
        3 => "⭐⭐⭐",
        2 => "⭐⭐",
        1 => "⭐",
        _ => "⭐⭐⭐",
    };

    format!(
        "✅ Feedback Recorded!\n\n\
         Customer: {}\n\
         Service: {}\n\
         Rating: {} {}\n\
         Comments: {}\n\
         Recorded At: {}\n\n\
         Thank you for your valuable feedback!\n\
         We continuously strive to improve our services.",
        customer_name,
        service_type,
        rating,
        emoji,
        comments,
        chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
    )
}

#[tool("Analyzes customer feedback and generates insights")]
fn analyze_feedback_trends() -> String {
    format!(
        "📊 Customer Feedback Analysis (Last 30 Days):\n\n\
         Overall Satisfaction: 4.6/5.0 ⭐\n\
         Total Reviews: 487\n\n\
         Top Positive Aspects:\n\
         - Professional Staff: 92%\n\
         - Quick Service: 88%\n\
         - Fair Pricing: 85%\n\
         - Clean Facility: 90%\n\n\
         Areas for Improvement:\n\
         - Waiting Time: 12% mentioned long waits\n\
         - Parking Space: 8% found limited parking\n\n\
         Trending Services:\n\
         1. AC Maintenance (Summer season)\n\
         2. Oil Change\n\
         3. Brake Service"
    )
}

// ============================================================================
// Main Application
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🚗 UAE Automotive Maintenance Service AI Agent");
    println!("================================================\n");
    dotenv::dotenv().ok();

    // ========================================================================
    // Sub-Agent 1: Diagnostic Agent
    // ========================================================================
    let diagnostic_agent = SubAgentConfig::new(
        "diagnostic-agent",
        "Expert automotive diagnostic specialist for UAE vehicles. Analyzes issues and recommends services.",
        "You are an expert automotive diagnostic specialist with 20+ years experience in UAE. \
         You understand all car makes and models sold in the UAE market. \
         Your role is to:\n\
         1. Ask relevant questions to understand the car issue\n\
         2. Use the diagnose_car_issue tool to analyze symptoms\n\
         3. Provide clear, professional recommendations\n\
         4. Calculate estimated costs for the customer\n\
         5. Consider UAE climate factors (heat, sand, humidity)\n\n\
         Always be thorough, professional, and customer-focused.",
    )
    .with_tools(vec![
        DiagnoseCarIssueTool::as_tool(),
        CalculateServiceCostTool::as_tool(),
    ]);

    // ========================================================================
    // Sub-Agent 2: Booking Agent
    // ========================================================================
    let booking_agent = SubAgentConfig::new(
        "booking-agent",
        "Service appointment booking specialist for UAE service centers.",
        "You are a booking specialist for automotive service centers across UAE. \
         Your role is to:\n\
         1. Check availability using check_availability tool\n\
         2. Book appointments using book_appointment tool\n\
         3. Provide clear information about service center locations\n\
         4. Offer flexible scheduling options\n\
         5. Consider customer preferences (location, time, service type)\n\n\
         Service Centers:\n\
         - Dubai: Al Quoz Industrial Area\n\
         - Abu Dhabi: Mussafah Industrial\n\
         - Sharjah: Industrial Area 6\n\n\
         Be friendly and accommodate customer needs.",
    )
    .with_tools(vec![
        CheckAvailabilityTool::as_tool(),
        BookAppointmentTool::as_tool(),
    ]);

    // ========================================================================
    // Sub-Agent 3: Ticketing Agent
    // ========================================================================
    let ticketing_agent = SubAgentConfig::new(
        "ticketing-agent",
        "Support ticket management specialist. Creates and tracks service tickets.",
        "You are a ticketing specialist managing customer service requests. \
         Your role is to:\n\
         1. Create detailed support tickets using create_support_ticket tool\n\
         2. Track ticket status using check_ticket_status tool\n\
         3. Ensure all customer issues are properly documented\n\
         4. Set appropriate priority levels (High/Medium/Low)\n\
         5. Provide ticket updates to customers\n\n\
         Priority Guidelines:\n\
         - High: Safety issues, vehicle not drivable\n\
         - Medium: Performance issues, scheduled maintenance\n\
         - Low: Cosmetic issues, general inquiries\n\n\
         Be organized and detail-oriented.",
    )
    .with_tools(vec![
        CreateSupportTicketTool::as_tool(),
        CheckTicketStatusTool::as_tool(),
    ]);

    // ========================================================================
    // Sub-Agent 4: Payment Agent
    // ========================================================================
    let payment_agent = SubAgentConfig::new(
        "payment-agent",
        "Secure payment processing specialist for UAE customers.",
        "You are a payment specialist handling transactions for automotive services. \
         Your role is to:\n\
         1. Generate secure payment links using generate_payment_link tool\n\
         2. Confirm payments using confirm_payment tool\n\
         3. Explain payment options clearly\n\
         4. Handle payment inquiries professionally\n\
         5. Ensure compliance with UAE payment regulations\n\n\
         Payment Methods:\n\
         - Credit/Debit Cards (Visa, Mastercard, AMEX)\n\
         - Apple Pay / Google Pay\n\
         - Bank Transfer (local UAE banks)\n\
         - Cash on Delivery\n\n\
         All prices include 5% VAT as per UAE law.\n\
         Be transparent and trustworthy.",
    )
    .with_tools(vec![
        GeneratePaymentLinkTool::as_tool(),
        ConfirmPaymentTool::as_tool(),
    ]);

    // ========================================================================
    // Sub-Agent 5: Notification Agent
    // ========================================================================
    let notification_agent = SubAgentConfig::new(
        "notification-agent",
        "Multi-channel notification specialist for customer communications.",
        "You are a notification specialist managing customer communications. \
         Your role is to:\n\
         1. Send timely notifications using send_notification tool\n\
         2. Choose appropriate channel (SMS, Email, WhatsApp)\n\
         3. Craft clear, concise messages\n\
         4. Ensure customers stay informed throughout their journey\n\
         5. Send confirmations, reminders, and updates\n\n\
         Channel Selection Guidelines:\n\
         - SMS: Urgent updates, appointment reminders\n\
         - Email: Detailed information, receipts, documentation\n\
         - WhatsApp: Quick updates, payment links, booking confirmations\n\n\
         Be timely and professional.",
    )
    .with_tools(vec![SendNotificationTool::as_tool()]);

    // ========================================================================
    // Sub-Agent 6: Feedback Agent
    // ========================================================================
    let feedback_agent = SubAgentConfig::new(
        "feedback-agent",
        "Customer satisfaction and feedback collection specialist.",
        "You are a feedback specialist focused on customer satisfaction. \
         Your role is to:\n\
         1. Collect customer feedback using collect_feedback tool\n\
         2. Analyze trends using analyze_feedback_trends tool\n\
         3. Ask relevant questions about service experience\n\
         4. Thank customers for their feedback\n\
         5. Identify areas for service improvement\n\n\
         Feedback Categories:\n\
         - Service Quality (technical work)\n\
         - Staff Professionalism\n\
         - Pricing Transparency\n\
         - Facility Cleanliness\n\
         - Wait Time\n\n\
         Be appreciative and encouraging.",
    )
    .with_tools(vec![
        CollectFeedbackTool::as_tool(),
        AnalyzeFeedbackTrendsTool::as_tool(),
    ]);

    // ========================================================================
    // Main Coordinator Agent
    // ========================================================================
    println!("🔧 Building main coordinator agent with 6 specialized sub-agents...\n");

    // Create explicit OpenAI model instance
    let openai_config = OpenAiConfig {
        api_key: std::env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY environment variable is required"),
        model: "gpt-4o-mini".to_string(),
        api_url: None,
    };
    let model = Arc::new(OpenAiChatModel::new(openai_config)?);

    let main_agent = ConfigurableAgentBuilder::new(
        "You are the main customer service coordinator for a premium automotive maintenance service in UAE. \
         You are professional, friendly, and fluent in English and Arabic.\n\n\
         Your role is to provide seamless service from first contact to feedback collection:\n\n\
         CUSTOMER JOURNEY STAGES:\n\
         1. GREETING & ISSUE IDENTIFICATION\n\
            - Greet customer warmly\n\
            - Ask about their vehicle and issue\n\
            - Delegate to diagnostic-agent for technical analysis\n\n\
         2. DIAGNOSIS & COST ESTIMATION\n\
            - Work with diagnostic-agent to understand the issue\n\
            - Provide clear cost estimates\n\
            - Explain recommended services\n\n\
         3. TICKET CREATION\n\
            - Delegate to ticketing-agent to create support ticket\n\
            - Ensure customer receives ticket number\n\n\
         4. APPOINTMENT BOOKING\n\
            - Delegate to booking-agent to schedule service\n\
            - Offer flexible timing and location options\n\n\
         5. PAYMENT PROCESSING\n\
            - Delegate to payment-agent to generate payment link\n\
            - Ensure secure payment process\n\
            - Send payment confirmation\n\n\
         6. NOTIFICATIONS\n\
            - Delegate to notification-agent for updates\n\
            - Send appointment reminders\n\
            - Notify when service is complete\n\n\
         7. SERVICE COMPLETION & FEEDBACK\n\
            - Confirm service completion\n\
            - Delegate to feedback-agent to collect satisfaction rating\n\
            - Thank customer and invite them back\n\n\
         UAE-SPECIFIC CONSIDERATIONS:\n\
         - Business hours: Saturday-Thursday (8 AM - 6 PM)\n\
         - Friday: Closed\n\
         - Currency: AED (UAE Dirham)\n\
         - All prices include 5% VAT\n\
         - Consider extreme heat and sand conditions\n\n\
         DELEGATION STRATEGY:\n\
         - Use diagnostic-agent for technical car issues\n\
         - Use booking-agent for appointments\n\
         - Use ticketing-agent for ticket management\n\
         - Use payment-agent for payments\n\
         - Use notification-agent for communications\n\
         - Use feedback-agent for satisfaction surveys\n\n\
         Always maintain a professional, helpful tone and ensure customer satisfaction.",
    )
    .with_model(model)
    .with_subagent_config([
        diagnostic_agent,
        booking_agent,
        ticketing_agent,
        payment_agent,
        notification_agent,
        feedback_agent,
    ])
    .with_auto_general_purpose(true) // Enable fallback agent
    .build()?;

    println!("✅ Main agent built successfully!\n");

    // ========================================================================
    // Test Complete Customer Journey
    // ========================================================================


    let customer_message =
        "Hi! My name is Ahmed. I have a 2019 Toyota Camry with 85,000 km. \
         Recently I've been hearing a strange grinding noise when I brake, \
         especially at high speeds. Also, my AC is not cooling as well as before. \
         Can you help me diagnose these issues and schedule a service? \
         I prefer Dubai location and I'm available this weekend. \
         Please also send me a quote and payment link. Thanks!";

    println!("👤 Customer Message:");
    println!("{}\n", customer_message);
    println!("🤖 Agent Response (Streaming - watch it type!):\n");

    // Use streaming interface
    let user_message = agents_sdk::messaging::AgentMessage {
        role: agents_sdk::messaging::MessageRole::User,
        content: agents_sdk::messaging::MessageContent::Text(customer_message.to_string()),
        metadata: None,
    };

    let mut stream = main_agent
        .handle_message_stream(user_message, Arc::new(AgentStateSnapshot::default()))
        .await?;

    std::io::Write::flush(&mut std::io::stdout()).unwrap();

    let mut full_response = String::new();

    while let Some(chunk_result) = stream.next().await {
        match chunk_result? {
            StreamChunk::TextDelta(delta) => {
                // Print the delta as it arrives
                print!("{}", delta);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_response.push_str(&delta);
            }
            StreamChunk::Done { message } => {
                // Stream complete
                println!("\n");
                if full_response.is_empty() {
                    // If we didn't get any deltas, use the final message
                    if let agents_sdk::messaging::MessageContent::Text(text) = message.content {
                        full_response = text;
                    }
                }
                break;
            }
            StreamChunk::Error(error) => {
                eprintln!("\n❌ Stream error: {}", error);
                break;
            }
        }
    }

    println!("\n✅ Complete Response Received ({} characters)\n", full_response.len());

    println!("{}\n", "=".repeat(60));
    println!("🎉 Demo Completed Successfully!\n");
    Ok(())
}