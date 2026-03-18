use anyhow::Result;
use chrono::{Duration, Local, NaiveDate};
use clap::{Parser, Subcommand};
use uuid::Uuid;

use rahd_ai::NlEventParser;
use rahd_core::{Contact, Event, EventFilter, Reminder, ReminderMethod};
use rahd_schedule::Scheduler;
use rahd_server::AppState;
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
        /// Add a reminder N minutes before the event
        #[arg(long)]
        remind: Option<u32>,
    },
    /// Edit an existing event
    Edit {
        /// Event ID
        event_id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New start time (ISO 8601 or YYYY-MM-DD HH:MM)
        #[arg(long)]
        start: Option<String>,
        /// New end time (ISO 8601 or YYYY-MM-DD HH:MM)
        #[arg(long)]
        end: Option<String>,
        /// New location
        #[arg(long)]
        location: Option<String>,
        /// Add a reminder N minutes before
        #[arg(long)]
        remind: Option<u32>,
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
    /// Show upcoming events with reminders due soon
    Upcoming {
        /// Show events within the next N minutes (default: 60)
        #[arg(long, default_value = "60")]
        within: u32,
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
    /// Import events from ICS file or contacts from vCard file
    Import {
        /// Path to .ics or .vcf file
        file: String,
    },
    /// Export events to ICS or contacts to vCard
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
    /// Start the daimon API server
    Serve {
        /// Address to bind (default: 127.0.0.1:8090)
        #[arg(long, default_value = "127.0.0.1:8090")]
        addr: String,
    },
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
    /// Delete a contact
    Delete {
        /// Contact ID
        contact_id: String,
    },
}

#[derive(Subcommand)]
enum ExportCommands {
    /// Export all events as ICS
    Events {
        /// Output file path (default: stdout)
        #[arg(long, short)]
        output: Option<String>,
    },
    /// Export all contacts as vCard
    Contacts {
        /// Output file path (default: stdout)
        #[arg(long, short)]
        output: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
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
        Commands::Add {
            description,
            remind,
        } => {
            let parser = NlEventParser::new();
            let parsed = parser.parse_event(&description)?;
            let now = chrono::Utc::now();
            let (start, end) = parsed.to_datetimes(now);
            let reminders = remind
                .map(|mins| {
                    vec![Reminder {
                        minutes_before: mins,
                        method: ReminderMethod::Notification,
                    }]
                })
                .unwrap_or_default();
            let event = Event {
                id: Uuid::new_v4(),
                title: parsed.title.clone(),
                description: Some(description),
                start,
                end,
                location: parsed.location.clone(),
                attendees: parsed.attendees.clone(),
                recurrence: None,
                reminders,
                calendar_id: "default".to_string(),
                created_at: now,
                updated_at: now,
            };
            store.add_event(&event)?;
            println!("Created: {event}");
            println!("ID: {}", event.id);
        }
        Commands::Edit {
            event_id,
            title,
            start,
            end,
            location,
            remind,
        } => {
            let id: Uuid = event_id.parse()?;
            let Some(mut event) = store.get_event(id)? else {
                println!("Event not found.");
                return Ok(());
            };
            if let Some(t) = title {
                event.title = t;
            }
            if let Some(s) = start {
                event.start = parse_flexible_datetime(&s)?;
            }
            if let Some(e) = end {
                event.end = parse_flexible_datetime(&e)?;
            }
            if let Some(loc) = location {
                event.location = Some(loc);
            }
            if let Some(mins) = remind {
                event.reminders.push(Reminder {
                    minutes_before: mins,
                    method: ReminderMethod::Notification,
                });
            }
            event.updated_at = chrono::Utc::now();
            store.update_event(&event)?;
            println!("Updated: {event}");
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
                let end = (now.date_naive() + Duration::days(7))
                    .and_hms_opt(23, 59, 59)
                    .unwrap();
                EventFilter {
                    from: Some(start.and_utc()),
                    to: Some(end.and_utc()),
                    ..Default::default()
                }
            } else if month {
                let start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
                let end = (now.date_naive() + Duration::days(30))
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
                    if !event.reminders.is_empty() {
                        for r in &event.reminders {
                            println!("Reminder:    {} min before", r.minutes_before);
                        }
                    }
                    if let Some(rec) = &event.recurrence {
                        println!("Recurrence:  {rec}");
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
        Commands::Upcoming { within } => {
            let now = chrono::Utc::now();
            let horizon = now + Duration::minutes(within as i64);
            let filter = EventFilter {
                from: Some(now),
                to: Some(horizon),
                ..Default::default()
            };
            let events = store.list_events(&filter)?;
            if events.is_empty() {
                println!("No upcoming events in the next {within} minutes.");
            } else {
                println!("Upcoming events (next {within} min):");
                for event in &events {
                    let mins_until = (event.start - now).num_minutes();
                    let reminder_due = event
                        .reminders
                        .iter()
                        .any(|r| mins_until <= r.minutes_before as i64);
                    let alert = if reminder_due { " [!]" } else { "" };
                    println!("  {} — {} (in {} min){alert}", event.id, event, mins_until);
                }
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
            ContactsCommands::Delete { contact_id } => {
                let id: Uuid = contact_id.parse()?;
                if store.delete_contact(id)? {
                    println!("Deleted contact {id}.");
                } else {
                    println!("Contact not found.");
                }
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
        Commands::Serve { addr } => {
            let state = AppState::new(store);
            rahd_server::serve(state, &addr).await?;
        }
        Commands::Import { file } => {
            let content = std::fs::read_to_string(&file)?;
            if file.ends_with(".ics") || file.ends_with(".ical") {
                let events = rahd_core::events_from_ics(&content);
                for event in &events {
                    store.add_event(event)?;
                }
                println!("Imported {} event(s) from {file}.", events.len());
            } else if file.ends_with(".vcf") || file.ends_with(".vcard") {
                let contacts = rahd_core::contacts_from_vcard(&content);
                for contact in &contacts {
                    store.add_contact(contact)?;
                }
                println!("Imported {} contact(s) from {file}.", contacts.len());
            } else {
                println!("Unsupported file format. Use .ics for events or .vcf for contacts.");
            }
        }
        Commands::Export { command } => match command {
            ExportCommands::Events { output } => {
                let events = store.list_events(&EventFilter::default())?;
                let ics = rahd_core::events_to_ics(&events);
                if let Some(path) = output {
                    std::fs::write(&path, &ics)?;
                    println!("Exported {} event(s) to {path}.", events.len());
                } else {
                    print!("{ics}");
                }
            }
            ExportCommands::Contacts { output } => {
                let contacts = store.list_contacts()?;
                let vcard = rahd_core::contacts_to_vcard(&contacts);
                if let Some(path) = output {
                    std::fs::write(&path, &vcard)?;
                    println!("Exported {} contact(s) to {path}.", contacts.len());
                } else {
                    print!("{vcard}");
                }
            }
        },
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

/// Parse a datetime from various formats.
fn parse_flexible_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    // Try RFC 3339 first
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&chrono::Utc));
    }
    // Try "YYYY-MM-DD HH:MM"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M") {
        return Ok(dt.and_utc());
    }
    // Try "YYYY-MM-DDTHH:MM"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M") {
        return Ok(dt.and_utc());
    }
    anyhow::bail!("could not parse datetime: {s} (use YYYY-MM-DD HH:MM or ISO 8601)")
}
