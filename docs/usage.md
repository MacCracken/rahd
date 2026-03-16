# Usage Guide

## Adding events

Use natural language to create events:

```bash
rahd add "lunch with Sam tomorrow at noon"
rahd add "meeting with Bob on Friday at 3pm for 1 hour"
rahd add "dentist appointment March 20 at 10am"
rahd add "standup at 9am" --remind 15
```

The parser understands:
- **Dates**: `today`, `tomorrow`, day names (`monday`, `friday`)
- **Times**: `noon`, `midnight`, `3pm`, `3:30pm`, `15:00`
- **Duration**: `for 2 hours`, `for 30 minutes`
- **Attendees**: `with Sam`, `with Alice Bob`
- **Location**: `at The Restaurant` (when not a time)

## Editing events

```bash
rahd edit <event-id> --title "New title"
rahd edit <event-id> --start "2026-03-20 14:00"
rahd edit <event-id> --location "Room 42"
rahd edit <event-id> --remind 30
```

## Viewing events

```bash
rahd list              # all events
rahd list --today      # today only
rahd list --week       # next 7 days
rahd list --month      # next 30 days
rahd show <event-id>   # full event details
```

## Upcoming reminders

```bash
rahd upcoming                # events in next 60 minutes
rahd upcoming --within 120   # events in next 2 hours
```

Events with reminders due show a `[!]` alert marker.

## Contacts

```bash
rahd contacts add "Sam Wilson" --email sam@example.com --phone 555-1234
rahd contacts list
rahd contacts delete <contact-id>
```

## Scheduling

```bash
rahd free                     # free slots today (9am-5pm)
rahd free --date 2026-03-20   # free slots on a specific date
rahd conflicts                # detect overlapping events
```

## Import & Export

### ICS (iCalendar)

```bash
rahd import calendar.ics                # import events from ICS file
rahd export events -o calendar.ics      # export all events to ICS
rahd export events                      # export to stdout
```

### vCard

```bash
rahd import contacts.vcf                # import contacts from vCard
rahd export contacts -o contacts.vcf    # export all contacts to vCard
rahd export contacts                    # export to stdout
```

## Database

By default, the database is stored at `~/.local/share/rahd/rahd.db`. Override with:

```bash
rahd --db /path/to/custom.db list
```
