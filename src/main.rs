use anyhow::Result;
use chrono::{Local, NaiveDate};
use clap::{Parser, Subcommand};
use uuid::Uuid;

use rahd_ai::NlEventParser;
use rahd_core::{Contact, Event, EventFilter};
use rahd_schedule::Scheduler;
use rahd_store::EventStore;

/// Rahd — AI-native calendar and contacts for AGNOS
///
/// Ruznam Ahd (Persian: daily record + Arabic: appointment)
#[derive(Parser)]
#[command(name = "rahd", version, about)]
struct Cli {
    /// Path to the database file
    #[arg(long, default_value = "~/.local/share/rahd/rahd.db")]
    db: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new event using natural language
    Add {
        /// Natural language event description (e.g. "lunch with Sam tomorrow at noon")
        description: String,
    },
    /// List events
    List {
        /// Show only today's events
        #[arg(long)]
        today: bool,
        /// Show this week's events
        #[arg(long)]
        week: bool,
        /// Show this month's events
        #[arg(long)]
        month: bool,
    },
    /// Show event details
    Show {
        /// Event ID
        event_id: String,
    },
    /// Delete an event
    Delete {
        /// Event ID
        event_id: String,
    },
    /// Manage contacts
    Contacts {
        #[command(subcommand)]
        command: ContactsCommands,
    },
    /// Show free time slots
    Free {
        /// Date to check (YYYY-MM-DD, default: today)
        #[arg(long)]
        date: Option<String>,
    },
    /// Show scheduling conflicts
    Conflicts,
}

#[derive(Subcommand)]
enum ContactsCommands {
    /// List all contacts
    List,
    /// Add a new contact
    Add {
        /// Contact name
        name: String,
        /// Email address
        #[arg(long)]
        email: Option<String>,
        /// Phone number
        #[arg(long)]
        phone: Option<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    let db_path = shellexpand(&cli.db);
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = EventStore::new(&db_path)?;

    match cli.command {
        Commands::Add { description } => {
            let parser = NlEventParser::new();
            let parsed = parser.parse_event(&description)?;
            let now = chrono::Utc::now();
            let (start, end) = parsed.to_datetimes(now);
            let event = Event {
                id: Uuid::new_v4(),
                title: parsed.title.clone(),
                description: Some(description),
                start,
                end,
                location: parsed.location.clone(),
                attendees: parsed.attendees.clone(),
                recurrence: None,
                reminders: vec![],
                calendar_id: "default".to_string(),
                created_at: now,
                updated_at: now,
            };
            store.add_event(&event)?;
            println!("Created: {event}");
            println!("ID: {}", event.id);
        }
        Commands::List { today, week, month } => {
            let now = Local::now();
            let filter = if today {
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let end = now.date_naive().and_hms_opt(23, 59, 59).unwrap();
                EventFilter {
                    from: Some(start.and_utc()),
                    to: Some(end.and_utc()),
                    ..Default::default()
                }
            } else if week {
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let end = (now.date_naive() + chrono::Duration::days(7))
                    .and_hms_opt(23, 59, 59)
                    .unwrap();
                EventFilter {
                    from: Some(start.and_utc()),
                    to: Some(end.and_utc()),
                    ..Default::default()
                }
            } else if month {
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let end = (now.date_naive() + chrono::Duration::days(30))
                    .and_hms_opt(23, 59, 59)
                    .unwrap();
                EventFilter {
                    from: Some(start.and_utc()),
                    to: Some(end.and_utc()),
                    ..Default::default()
                }
            } else {
                EventFilter::default()
            };
            let events = store.list_events(&filter)?;
            if events.is_empty() {
                println!("No events found.");
            } else {
                for event in &events {
                    println!("  {} — {}", event.id, event);
                }
            }
        }
        Commands::Show { event_id } => {
            let id: Uuid = event_id.parse()?;
            match store.get_event(id)? {
                Some(event) => {
                    println!("Title:       {}", event.title);
                    println!("Start:       {}", event.start);
                    println!("End:         {}", event.end);
                    if let Some(loc) = &event.location {
                        println!("Location:    {loc}");
                    }
                    if !event.attendees.is_empty() {
                        println!("Attendees:   {}", event.attendees.join(", "));
                    }
                    if let Some(desc) = &event.description {
                        println!("Description: {desc}");
                    }
                    println!("Calendar:    {}", event.calendar_id);
                    println!("ID:          {}", event.id);
                }
                None => println!("Event not found."),
            }
        }
        Commands::Delete { event_id } => {
            let id: Uuid = event_id.parse()?;
            if store.delete_event(id)? {
                println!("Deleted event {id}.");
            } else {
                println!("Event not found.");
            }
        }
        Commands::Contacts { command } => match command {
            ContactsCommands::List => {
                let contacts = store.list_contacts()?;
                if contacts.is_empty() {
                    println!("No contacts found.");
                } else {
                    for c in &contacts {
                        let email = c.email.as_deref().unwrap_or("-");
                        let phone = c.phone.as_deref().unwrap_or("-");
                        println!("  {} — {} | {} | {}", c.id, c.name, email, phone);
                    }
                }
            }
            ContactsCommands::Add { name, email, phone } => {
                let contact = Contact {
                    id: Uuid::new_v4(),
                    name: name.clone(),
                    email,
                    phone,
                    organization: None,
                    notes: None,
                    created_at: chrono::Utc::now(),
                };
                store.add_contact(&contact)?;
                println!("Added contact: {name} ({})", contact.id);
            }
        },
        Commands::Free { date } => {
            let target = match date {
                Some(d) => d.parse::<NaiveDate>()?,
                None => Local::now().date_naive(),
            };
            let events = store.list_events(&EventFilter::default())?;
            let scheduler = Scheduler::new();
            let slots = scheduler.find_free_slots(&events, target, 9, 17);
            if slots.is_empty() {
                println!("No free slots on {target}.");
            } else {
                println!("Free slots on {target}:");
                for slot in &slots {
                    println!(
                        "  {} - {}",
                        slot.start.format("%H:%M"),
                        slot.end.format("%H:%M")
                    );
                }
            }
        }
        Commands::Conflicts => {
            let events = store.list_events(&EventFilter::default())?;
            let scheduler = Scheduler::new();
            let conflicts = scheduler.find_conflicts(&events);
            if conflicts.is_empty() {
                println!("No scheduling conflicts found.");
            } else {
                println!("{} conflict(s) found:", conflicts.len());
                for c in &conflicts {
                    println!(
                        "  {} overlaps with {} ({} - {})",
                        c.event_a,
                        c.event_b,
                        c.overlap.start.format("%H:%M"),
                        c.overlap.end.format("%H:%M")
                    );
                }
            }
        }
    }

    Ok(())
}

fn shellexpand(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{home}/{rest}");
    }
    path.to_string()
}
