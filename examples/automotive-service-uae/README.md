# 🚗 UAE Automotive Maintenance Service - Deep Agent Example

A comprehensive demonstration of the **Rust Deep Agents SDK** implementing a complete automotive maintenance service system for the UAE market.

## 📋 Overview

This example showcases a production-ready multi-agent system that handles the entire customer journey for automotive maintenance services, from initial contact to feedback collection.

### Customer Journey Stages

```
Customer Says "Hi"
    ↓
1. Greeting & Issue Identification
    ↓
2. Diagnosis & Cost Estimation (Diagnostic Agent)
    ↓
3. Support Ticket Creation (Ticketing Agent)
    ↓
4. Appointment Booking (Booking Agent)
    ↓
5. Payment Processing (Payment Agent)
    ↓
6. Notifications & Updates (Notification Agent)
    ↓
7. Service Completion & Feedback (Feedback Agent)
    ↓
Complete!
```

## 🏗️ System Architecture

### Main Coordinator Agent
The main agent orchestrates the entire customer experience, delegating tasks to specialized sub-agents while maintaining conversation flow.

### 6 Specialized Sub-Agents

| Sub-Agent | Purpose | Tools |
|-----------|---------|-------|
| **🔧 Diagnostic Agent** | Analyzes car issues, provides recommendations | `diagnose_car_issue`, `calculate_service_cost` |
| **📅 Booking Agent** | Manages appointment scheduling | `check_availability`, `book_appointment` |
| **🎫 Ticketing Agent** | Creates and tracks support tickets | `create_support_ticket`, `check_ticket_status` |
| **💳 Payment Agent** | Processes payments securely | `generate_payment_link`, `confirm_payment` |
| **📱 Notification Agent** | Multi-channel communications | `send_notification` |
| **⭐ Feedback Agent** | Collects satisfaction ratings | `collect_feedback`, `analyze_feedback_trends` |

### 13 Specialized Tools

All tools are defined using the `#[tool]` macro for automatic schema generation:

#### Diagnostic Tools
- `diagnose_car_issue` - AI-powered diagnosis based on symptoms
- `calculate_service_cost` - Estimates costs for services

#### Ticketing Tools
- `create_support_ticket` - Creates tracked support tickets
- `check_ticket_status` - Monitors ticket progress

#### Booking Tools
- `check_availability` - Checks service center availability
- `book_appointment` - Schedules service appointments

#### Payment Tools
- `generate_payment_link` - Creates secure payment links
- `confirm_payment` - Processes payment confirmations

#### Notification Tools
- `send_notification` - Sends SMS/Email/WhatsApp notifications

#### Feedback Tools
- `collect_feedback` - Gathers customer satisfaction ratings
- `analyze_feedback_trends` - Analyzes feedback patterns

## 🌍 UAE-Specific Features

- **Currency**: All prices in AED (UAE Dirham) with 5% VAT
- **Business Hours**: Saturday-Thursday (Friday closed)
- **Service Centers**: Dubai, Abu Dhabi, Sharjah
- **Climate Considerations**: Accounts for extreme heat, sand, humidity
- **Payment Methods**: UAE banks, local payment options
- **Multi-Channel**: SMS (Etisalat/Du), Email, WhatsApp

## 🚀 Running the Example

### Prerequisites

1. Set your OpenAI API key:
```bash
export OPENAI_API_KEY="your-api-key-here"
```

Or create a `.env` file:
```bash
echo "OPENAI_API_KEY=your-api-key-here" > .env
```

**Note**: The system uses OpenAI GPT-4o-mini by default for cost-effective operation. You can override this with any model using `.with_model()` in your code.

### Build & Run

```bash
# Build the example
cargo build --package automotive-service-uae

# Run the example
cargo run --package automotive-service-uae
```

### Expected Output

The system will demonstrate a complete customer journey:

1. Customer greets and describes issue (brake noise + AC problem)
2. Main agent delegates to Diagnostic Agent for analysis
3. Diagnostic Agent uses tools to diagnose and estimate costs
4. Ticketing Agent creates support ticket
5. Booking Agent schedules appointment
6. Payment Agent generates payment link
7. Notification Agent sends confirmations
8. Feedback Agent collects satisfaction rating

## 💡 Key Concepts Demonstrated

### 1. Deep Agents Architecture
- Main coordinator with multiple specialized sub-agents
- Proper delegation and context quarantine
- Each sub-agent is a full AI agent with its own tools and instructions

### 2. Tool System
- Clean `#[tool]` macro usage for automatic schema generation
- Type-safe tool definitions with Rust types
- Automatic JSON Schema generation for LLMs

### 3. Sub-Agent Coordination
- Main agent knows when to delegate to specialists
- Sub-agents have focused responsibilities
- Clear handoffs between agents

### 4. State Management
- Shared state between agents
- Tool results update state automatically
- State snapshot pattern for concurrency

### 5. UAE Business Logic
- Local currency, VAT calculations
- Business hours, holidays
- Local payment methods and regulations
- Regional service center locations

## 🧠 How Sub-Agents Work

### LLM Inheritance

**By default, sub-agents inherit the parent agent's LLM.**

In this example:
```rust
// Main agent uses OpenAI GPT-4o-mini
let openai_config = OpenAiConfig {
    api_key: std::env::var("OPENAI_API_KEY")?,
    model: "gpt-4o-mini".to_string(),
    api_url: None,
};
let model = Arc::new(OpenAiChatModel::new(openai_config)?);

let main_agent = ConfigurableAgentBuilder::new(...)
    .with_model(model)  // <-- Parent's model
    .with_subagent_config([diagnostic_agent, booking_agent, ...])
    .build()?;
```

**All 6 sub-agents (diagnostic, booking, ticketing, payment, notification, feedback) inherit GPT-4o-mini from the main agent.**

### Execution Flow

```
┌─────────────┐
│ Main Agent  │
│ (GPT-4o-mini)│
└──────┬──────┘
       │ Main agent decides: "I need diagnostic help"
       │
       ▼
┌──────────────────────────────────────────┐
│ Main agent calls:                         │
│ task(                                     │
│   description="Customer has brake noise", │
│   subagent_type="diagnostic-agent"        │
│ )                                         │
└──────────────────────────────────────────┘
       │
       ▼
┌────────────────┐
│ TaskRouterTool │  <-- Middleware handles this
└───────┬────────┘
        │ 1. Look up "diagnostic-agent" in registry
        │ 2. Create fresh conversation with description
        │
        ▼
┌──────────────────┐
│ Diagnostic Agent │
│ (GPT-4o-mini)    │ <-- Inherited from parent
│                  │
│ Tools:           │
│ - diagnose_car   │
│ - calc_cost      │
│                  │
│ Instructions:    │
│ "You are expert" │
│ "20+ years UAE"  │
└────────┬─────────┘
         │ Executes independently
         │ Can call its own tools
         │ Has its own conversation
         │
         ▼
┌─────────────────────┐
│ Sub-agent Response  │
│ "Based on symptoms, │
│  likely worn brake  │
│  pads. Cost: 800AED"│
└──────────┬──────────┘
           │
           ▼
┌──────────────────┐
│ Return to Main   │
│ as Tool Result   │
└──────────────────┘
```

### Context Quarantine

**Main agent sees:**
```
User: "I have brake noise"
Assistant: [calls task tool with diagnostic-agent]
Tool Result: "Based on symptoms, likely worn brake pads. Cost: 800 AED"
Assistant: "Based on the diagnosis, you have worn brake pads..."
```

**Main agent does NOT see:**
- Sub-agent calling `diagnose_car_issue` tool
- Sub-agent's internal reasoning
- Sub-agent's intermediate steps

This keeps the main agent's context clean and prevents token bloat!

### Overriding the Model Per Sub-Agent

If you want a sub-agent to use a **different** model (e.g., Claude for complex reasoning):

```rust
// Create a different model for this specific sub-agent
let claude_config = AnthropicConfig {
    api_key: std::env::var("ANTHROPIC_API_KEY")?,
    model: "claude-sonnet-4-20250514".to_string(),
    max_output_tokens: 8000,
    api_url: None,
    api_version: None,
};
let claude_model = Arc::new(AnthropicMessagesModel::new(claude_config)?);

let diagnostic_agent = SubAgentConfig::new(
    "diagnostic-agent",
    "Expert automotive diagnostic specialist",
    "You are an expert..."
)
.with_model(claude_model)  // <-- Override with Claude!
.with_tools(vec![
    DiagnoseCarIssueTool::as_tool(),
    CalculateServiceCostTool::as_tool(),
]);
```

Now you have:
- **Main agent**: GPT-4o-mini (cost-effective coordination)
- **Diagnostic sub-agent**: Claude Sonnet 4 (complex reasoning)
- **Other sub-agents**: GPT-4o-mini (inherited)

This is powerful for:
- ✅ Using cheaper models for simple sub-agents
- ✅ Using specialized models for complex reasoning
- ✅ Cost optimization per sub-agent
- ✅ Mixing providers (OpenAI + Anthropic + Gemini)

## 📊 Example Customer Interaction

```rust
let customer_message =
    "Hi! My name is Ahmed. I have a 2019 Toyota Camry with 85,000 km. \
     Recently I've been hearing a strange grinding noise when I brake, \
     especially at high speeds. Also, my AC is not cooling as well as before. \
     Can you help me diagnose these issues and schedule a service? \
     I prefer Dubai location and I'm available this weekend. \
     Please also send me a quote and payment link. Thanks!";
```

The system will:
1. ✅ Diagnose both issues (brake + AC)
2. ✅ Estimate costs (considering vehicle age, brand, mileage)
3. ✅ Create support ticket with unique ID
4. ✅ Check Dubai service center availability for weekend
5. ✅ Book appointment at preferred time
6. ✅ Generate secure payment link
7. ✅ Send WhatsApp confirmation with all details
8. ✅ Follow up for feedback after service

## 🔧 Customization

### Adding New Tools

```rust
#[tool("Description of what the tool does")]
fn my_custom_tool(param1: String, param2: i32) -> String {
    // Tool implementation
    format!("Result: {} - {}", param1, param2)
}
```

### Adding New Sub-Agents

```rust
let custom_agent = SubAgentConfig::new(
    "custom-agent",
    "Description for main agent",
    "Detailed instructions for this agent",
)
.with_tools(vec![MyCustomToolTool::as_tool()]);
```

### Modifying Business Logic

Update tool implementations to match your business rules:
- Pricing calculations
- Service center locations
- Appointment scheduling logic
- Payment processing flow

## 🏢 Production Considerations

For production deployment, consider:

### 1. Database Integration
Replace simulated data with real database calls:
- Customer records
- Service history
- Appointment calendar
- Payment transactions

### 2. External Service Integration
- Real SMS gateway (Twilio, MessageBird)
- Email service (SendGrid, AWS SES)
- WhatsApp Business API
- Payment gateway (Stripe, Checkout.com)

### 3. Security
- Secure API key storage
- PCI compliance for payments
- Customer data encryption
- Audit logging

### 4. Scalability
- Async processing for notifications
- Queue system for high volume
- Caching for frequently accessed data
- Load balancing for service centers

### 5. Monitoring
- Agent performance metrics
- Tool execution tracking
- Customer satisfaction trends
- Error rate monitoring

## 📚 Learning Resources

### Rust Deep Agents Concepts
- **Deep Agents**: Multi-layered agent architecture for complex tasks
- **Sub-Agents**: Specialized agents with focused responsibilities
- **Tool System**: Type-safe function-to-LLM-tool conversion
- **State Management**: Shared state with snapshot pattern

### Related Examples
- `examples/subagent-demo/` - Basic sub-agent demonstration
- `examples/unified-sdk-test/` - Tool macro usage
- `examples/simple-agent/` - Single agent basics

### Documentation
- See `docs/deep-agents.md` for architecture details
- See Python SDK comparison in `docs/reference/deep-agents.md`

## 🎯 Business Value

This example demonstrates how to:
- **Reduce Response Time**: Automated diagnosis and scheduling
- **Improve Accuracy**: AI-powered issue detection
- **Enhance Customer Experience**: Seamless journey from inquiry to completion
- **Increase Efficiency**: Parallel processing of multiple requests
- **Scale Operations**: Handle more customers without proportional staff increase
- **Gather Insights**: Automated feedback collection and analysis

## 🤝 Contributing

Improvements welcome:
- Additional automotive diagnostic logic
- More payment gateway integrations
- Enhanced notification templates
- Multi-language support (Arabic)
- Integration with real automotive APIs

## 📄 License

Same as the main Rust Deep Agents SDK project.

## 🙏 Acknowledgements

This example is inspired by:
- Claude Code architecture
- Python Deep Agents SDK
- Real-world automotive service operations in UAE

---

Built with ❤️ using Rust Deep Agents SDK