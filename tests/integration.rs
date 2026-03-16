//! Integration tests — end-to-end CLI round-trips using an in-memory-like temp DB.

use std::process::Command;

fn rahd_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rahd"));
    let tmp = std::env::temp_dir().join(format!("rahd-test-{}.db", std::process::id()));
    cmd.arg("--db").arg(&tmp);
    cmd
}

fn rahd_with_db(db: &std::path::Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rahd"));
    cmd.arg("--db").arg(db);
    cmd
}

#[test]
fn add_and_list_event() {
    let tmp = std::env::temp_dir().join(format!("rahd-int-{}-1.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp);

    // Add
    let output = rahd_with_db(&tmp)
        .args(["add", "lunch with Sam tomorrow at noon"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created:"), "stdout: {stdout}");
    assert!(stdout.contains("ID:"));

    // Extract UUID from "ID: <uuid>" line
    let id = stdout
        .lines()
        .find(|l| l.starts_with("ID:"))
        .unwrap()
        .trim_start_matches("ID: ")
        .trim();

    // Show
    let output = rahd_with_db(&tmp).args(["show", id]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lunch"), "show stdout: {stdout}");

    // List
    let output = rahd_with_db(&tmp).args(["list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lunch"), "list stdout: {stdout}");

    // Delete
    let output = rahd_with_db(&tmp).args(["delete", id]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Deleted"), "delete stdout: {stdout}");

    // Verify deleted
    let output = rahd_with_db(&tmp).args(["show", id]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("not found"), "show-after-delete: {stdout}");

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn add_and_edit_event() {
    let tmp = std::env::temp_dir().join(format!("rahd-int-{}-2.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp);

    let output = rahd_with_db(&tmp)
        .args(["add", "meeting at 3pm"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let id = stdout
        .lines()
        .find(|l| l.starts_with("ID:"))
        .unwrap()
        .trim_start_matches("ID: ")
        .trim();

    // Edit title
    let output = rahd_with_db(&tmp)
        .args(["edit", id, "--title", "Updated meeting"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated"), "edit stdout: {stdout}");

    // Verify
    let output = rahd_with_db(&tmp).args(["show", id]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Updated meeting"),
        "show after edit: {stdout}"
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn contacts_round_trip() {
    let tmp = std::env::temp_dir().join(format!("rahd-int-{}-3.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp);

    // Add
    let output = rahd_with_db(&tmp)
        .args([
            "contacts",
            "add",
            "Alice Smith",
            "--email",
            "alice@example.com",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Added contact"), "add: {stdout}");

    // List
    let output = rahd_with_db(&tmp)
        .args(["contacts", "list"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Alice Smith"), "list: {stdout}");
    assert!(stdout.contains("alice@example.com"));

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn free_slots_and_conflicts() {
    let tmp = std::env::temp_dir().join(format!("rahd-int-{}-4.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp);

    // Free slots on empty calendar
    let output = rahd_with_db(&tmp)
        .args(["free", "--date", "2026-04-01"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Free slots") || stdout.contains("09:00"),
        "free: {stdout}"
    );

    // Conflicts on empty calendar
    let output = rahd_with_db(&tmp).args(["conflicts"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No scheduling conflicts"),
        "conflicts: {stdout}"
    );

    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn import_export_ics_round_trip() {
    let tmp_db = std::env::temp_dir().join(format!("rahd-int-{}-5.db", std::process::id()));
    let tmp_ics = std::env::temp_dir().join(format!("rahd-int-{}-5.ics", std::process::id()));
    let _ = std::fs::remove_file(&tmp_db);
    let _ = std::fs::remove_file(&tmp_ics);

    // Add events
    rahd_with_db(&tmp_db)
        .args(["add", "morning standup at 9am"])
        .output()
        .unwrap();
    rahd_with_db(&tmp_db)
        .args(["add", "lunch at noon"])
        .output()
        .unwrap();

    // Export
    let output = rahd_with_db(&tmp_db)
        .args(["export", "events", "-o", tmp_ics.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Exported 2 event(s)"), "export: {stdout}");

    // Verify ICS file content
    let ics_content = std::fs::read_to_string(&tmp_ics).unwrap();
    assert!(ics_content.contains("BEGIN:VCALENDAR"));
    assert!(ics_content.contains("SUMMARY:morning standup"));

    // Import into fresh DB
    let tmp_db2 = std::env::temp_dir().join(format!("rahd-int-{}-5b.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp_db2);
    let output = rahd_with_db(&tmp_db2)
        .args(["import", tmp_ics.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Imported 2 event(s)"), "import: {stdout}");

    // Verify imported
    let output = rahd_with_db(&tmp_db2).args(["list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("standup") || stdout.contains("lunch"),
        "list after import: {stdout}"
    );

    let _ = std::fs::remove_file(&tmp_db);
    let _ = std::fs::remove_file(&tmp_db2);
    let _ = std::fs::remove_file(&tmp_ics);
}

#[test]
fn export_import_vcard_round_trip() {
    let tmp_db = std::env::temp_dir().join(format!("rahd-int-{}-6.db", std::process::id()));
    let tmp_vcf = std::env::temp_dir().join(format!("rahd-int-{}-6.vcf", std::process::id()));
    let _ = std::fs::remove_file(&tmp_db);
    let _ = std::fs::remove_file(&tmp_vcf);

    // Add contact
    rahd_with_db(&tmp_db)
        .args([
            "contacts",
            "add",
            "Bob Jones",
            "--email",
            "bob@example.com",
            "--phone",
            "555-9999",
        ])
        .output()
        .unwrap();

    // Export
    let output = rahd_with_db(&tmp_db)
        .args(["export", "contacts", "-o", tmp_vcf.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Exported 1 contact(s)"), "export: {stdout}");

    // Import into fresh DB
    let tmp_db2 = std::env::temp_dir().join(format!("rahd-int-{}-6b.db", std::process::id()));
    let _ = std::fs::remove_file(&tmp_db2);
    let output = rahd_with_db(&tmp_db2)
        .args(["import", tmp_vcf.to_str().unwrap()])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Imported 1 contact(s)"), "import: {stdout}");

    let _ = std::fs::remove_file(&tmp_db);
    let _ = std::fs::remove_file(&tmp_db2);
    let _ = std::fs::remove_file(&tmp_vcf);
}

#[test]
fn upcoming_shows_events() {
    let output = rahd_cmd()
        .args(["upcoming", "--within", "1"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // On a fresh DB, there should be no upcoming events
    assert!(stdout.contains("No upcoming events"), "upcoming: {stdout}");
}

#[test]
fn help_shows_commands() {
    let output = rahd_cmd().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("add"));
    assert!(stdout.contains("edit"));
    assert!(stdout.contains("list"));
    assert!(stdout.contains("upcoming"));
    assert!(stdout.contains("import"));
    assert!(stdout.contains("export"));
}
