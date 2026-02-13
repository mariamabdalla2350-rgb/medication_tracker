use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

#[derive(Debug, Clone)]
struct Medication {
    name: String,
    dosage: String,
    time_of_day: String,
    current_count: u32,
    total_prescribed: u32,
}

#[derive(Debug, Clone)]
struct DailyLog {
    date: String,
    taken: HashMap<String, bool>,
}

struct MedicationTracker {
    medications: HashMap<String, Medication>,
    daily_logs: HashMap<String, DailyLog>,
    patient_name: String,
    data_file: String,
    log_file: String,
}

impl MedicationTracker {
    fn new(patient_name: &str) -> Self {
        let data_file = format!("{}_meds.txt", patient_name);
        let log_file = format!("{}_logs.txt", patient_name);
        
        let mut tracker = MedicationTracker {
            medications: HashMap::new(),
            daily_logs: HashMap::new(),
            patient_name: patient_name.to_string(),
            data_file,
            log_file,
        };
        tracker.load_data();
        tracker.load_logs();
        tracker
    }

    fn add_medication(&mut self, name: String, dosage: String, time_of_day: String, count: u32) {
        let med = Medication {
            name: name.clone(),
            dosage,
            time_of_day,
            current_count: count,
            total_prescribed: count,
        };
        self.medications.insert(name, med);
        self.save_data();
    }

    fn mark_taken(&mut self, med_name: &str, date: &str, taken: bool) -> Result<(), String> {
        if !self.medications.contains_key(med_name) {
            return Err("Medication not found".to_string());
        }

        let log = self.daily_logs.entry(date.to_string()).or_insert(DailyLog {
            date: date.to_string(),
            taken: HashMap::new(),
        });
        
        log.taken.insert(med_name.to_string(), taken);
        
        if taken {
            if let Some(med) = self.medications.get_mut(med_name) {
                if med.current_count > 0 {
                    med.current_count -= 1;
                }
            }
        }
        
        self.save_logs();
        self.save_data();
        Ok(())
    }

    fn check_today_status(&self, date: &str) -> Vec<(String, String, bool, String)> {
        let mut status = Vec::new();
        
        for (name, med) in &self.medications {
            let taken = self.daily_logs
                .get(date)
                .and_then(|log| log.taken.get(name))
                .copied()
                .unwrap_or(false);
            
            let reminder = if !taken {
                format!("REMINDER: Take {} at {}", name, med.time_of_day)
            } else {
                "Taken".to_string()
            };
            
            status.push((
                name.clone(),
                format!("{} ({})", med.dosage, med.time_of_day),
                taken,
                reminder
            ));
        }
        
        status.sort_by(|a, b| a.1.cmp(&b.1));
        status
    }

    fn get_missed_medications(&self, date: &str) -> Vec<String> {
        let mut missed = Vec::new();
        
        for (name, med) in &self.medications {
            let taken = self.daily_logs
                .get(date)
                .and_then(|log| log.taken.get(name))
                .copied()
                .unwrap_or(false);
            
            if !taken {
                missed.push(format!("{} at {}", name, med.time_of_day));
            }
        }
        
        missed
    }

    fn generate_weekly_summary(&self, week_start: &str) -> String {
        let mut summary = String::new();
        summary.push_str(&format!("\n========== WEEKLY SUMMARY FOR {} ==========\n", self.patient_name));
        summary.push_str(&format!("Week starting: {}\n\n", week_start));

        let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        
        for (med_name, med) in &self.medications {
            summary.push_str(&format!("MEDICATION: {} ({})\n", med_name, med.dosage));
            summary.push_str("Daily Record: ");
            
            let mut taken_count = 0;
            for (i, day) in days.iter().enumerate() {
                let date = format!("{}-{}", week_start, i);
                let taken = self.daily_logs
                    .get(&date)
                    .and_then(|log| log.taken.get(med_name))
                    .copied()
                    .unwrap_or(false);
                
                let symbol = if taken { "[X]" } else { "[ ]" };
                summary.push_str(&format!("{} {} ", day, symbol));
                
                if taken {
                    taken_count += 1;
                }
            }
            
            let percentage = (taken_count as f32 / 7.0) * 100.0;
            summary.push_str(&format!("\nAdherence: {}/7 days ({:.1}%)\n", taken_count, percentage));
            summary.push_str(&format!("Remaining: {} of {} doses\n\n", med.current_count, med.total_prescribed));
        }

        summary.push_str("DAILY OVERVIEW:\n");
        for (i, day) in days.iter().enumerate() {
            let date = format!("{}-{}", week_start, i);
            
            let total_meds = self.medications.len();
            let taken_meds = self.daily_logs
                .get(&date)
                .map(|log| log.taken.values().filter(|&&v| v).count())
                .unwrap_or(0);
            
            summary.push_str(&format!("{}: {}/{} medications taken", day, taken_meds, total_meds));
            
            if taken_meds < total_meds {
                let missed = self.get_missed_medications(&date);
                if !missed.is_empty() {
                    summary.push_str(&format!(" - MISSED: {}", missed.join(", ")));
                }
            }
            summary.push('\n');
        }

        summary.push_str("\n==========================================\n");
        summary
    }

    fn save_chart_to_file(&self, week_start: &str) -> Result<String, String> {
        let summary = self.generate_weekly_summary(week_start);
        let filename = format!("{}_weekly_report_{}.txt", self.patient_name, week_start);
        
        let mut file = File::create(&filename).map_err(|e| e.to_string())?;
        file.write_all(summary.as_bytes()).map_err(|e| e.to_string())?;
        
        Ok(filename)
    }

    fn save_data(&self) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.data_file)
            .expect("Cannot open meds file");
        
        for med in self.medications.values() {
            let line = format!("{},{},{},{},{}\n",
                med.name,
                med.dosage,
                med.time_of_day,
                med.current_count,
                med.total_prescribed
            );
            file.write_all(line.as_bytes()).expect("Write failed");
        }
    }

    fn load_data(&mut self) {
        if !Path::new(&self.data_file).exists() {
            return;
        }
        
        let file = File::open(&self.data_file).expect("Cannot open meds file");
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 5 {
                    let med = Medication {
                        name: parts[0].to_string(),
                        dosage: parts[1].to_string(),
                        time_of_day: parts[2].to_string(),
                        current_count: parts[3].parse().unwrap_or(0),
                        total_prescribed: parts[4].parse().unwrap_or(0),
                    };
                    self.medications.insert(med.name.clone(), med);
                }
            }
        }
    }

    fn save_logs(&self) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.log_file)
            .expect("Cannot open log file");
        
        for log in self.daily_logs.values() {
            for (med_name, taken) in &log.taken {
                let line = format!("{},{},{}\n",
                    log.date,
                    med_name,
                    if *taken { "1" } else { "0" }
                );
                file.write_all(line.as_bytes()).expect("Write failed");
            }
        }
    }

    fn load_logs(&mut self) {
        if !Path::new(&self.log_file).exists() {
            return;
        }
        
        let file = File::open(&self.log_file).expect("Cannot open log file");
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.split(',').collect();
                if parts.len() == 3 {
                    let date = parts[0].to_string();
                    let log = self.daily_logs.entry(date.clone()).or_insert(DailyLog {
                        date,
                        taken: HashMap::new(),
                    });
                    log.taken.insert(parts[1].to_string(), parts[2] == "1");
                }
            }
        }
    }

    fn list_medications(&self) -> Vec<String> {
        self.medications.values()
            .map(|med| format!("{} - {} at {} ({} left)", 
                med.name, med.dosage, med.time_of_day, med.current_count))
            .collect()
    }

    fn refill_medication(&mut self, name: &str, amount: u32) -> Result<(), String> {
        match self.medications.get_mut(name) {
            Some(med) => {
                med.current_count += amount;
                med.total_prescribed += amount;
                self.save_data();
                Ok(())
            }
            None => Err("Medication not found".to_string()),
        }
    }
}

fn get_today() -> String {
    "2024-W01-1".to_string()
}

fn get_week_start() -> String {
    "2024-W01".to_string()
}

fn clear_screen() {
    print!("\n{}\n", "=".repeat(50));
}

fn print_header(text: &str) {
    println!("\n{:=^50}", text);
}

fn wait_for_enter() {
    println!("\nPress ENTER to continue...");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
}

fn main() {
    clear_screen();
    print_header(" MEDICATION TRACKER FOR SENIORS ");
    
    println!("Enter patient name: ");
    let mut patient_name = String::new();
    io::stdin().read_line(&mut patient_name).unwrap();
    let patient_name = patient_name.trim().to_string();
    
    let mut tracker = MedicationTracker::new(&patient_name);
    let today = get_today();
    
    loop {
        clear_screen();
        print_header(&format!(" Hello, {} ", patient_name));
        
        let status = tracker.check_today_status(&today);
        let missed = tracker.get_missed_medications(&today);
        
        println!("TODAY: {}", today);
        println!("{}", "-".repeat(50));
        
        if !missed.is_empty() {
            println!("REMINDERS - Please take:");
            for reminder in &missed {
                println!("   * {}", reminder);
            }
        } else if !status.is_empty() {
            println!("All medications taken today!");
        } else {
            println!("No medications scheduled.");
        }
        
        println!("{}", "-".repeat(50));
        println!("MENU:");
        println!("1. View Today's Medications");
        println!("2. Mark Medication as Taken");
        println!("3. Mark Medication as Missed");
        println!("4. View All Medications");
        println!("5. Add New Medication");
        println!("6. Refill Medication");
        println!("7. View Weekly Summary");
        println!("8. Save Weekly Report to File");
        println!("9. Exit");
        println!("{}", "-".repeat(50));
        print!("Choice (1-9): ");
        
        io::stdout().flush().unwrap();
        let mut choice = String::new();
        io::stdin().read_line(&mut choice).unwrap();
        
        match choice.trim() {
            "1" => {
                clear_screen();
                print_header(" TODAY'S MEDICATIONS ");
                
                if status.is_empty() {
                    println!("No medications scheduled.");
                } else {
                    for (name, details, taken, reminder) in status {
                        let status_symbol = if taken { "[X] TAKEN" } else { "[ ] NOT TAKEN" };
                        println!("{}", name);
                        println!("   Status: {}", status_symbol);
                        println!("   Details: {}", details);
                        if !taken {
                            println!("   *** {}", reminder);
                        }
                        println!();
                    }
                }
                wait_for_enter();
            }
            
            "2" => {
                clear_screen();
                print_header(" MARK AS TAKEN ");
                
                let meds: Vec<_> = tracker.medications.keys().cloned().collect();
                if meds.is_empty() {
                    println!("No medications to mark.");
                    wait_for_enter();
                    continue;
                }
                
                for (i, med) in meds.iter().enumerate() {
                    println!("{}. {}", i + 1, med);
                }
                
                print!("Enter number: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                if let Ok(num) = input.trim().parse::<usize>() {
                    if num > 0 && num <= meds.len() {
                        let med_name = &meds[num - 1];
                        match tracker.mark_taken(med_name, &today, true) {
                            Ok(_) => println!("Recorded: {} taken", med_name),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        println!("Invalid selection.");
                    }
                }
                wait_for_enter();
            }
            
            "3" => {
                clear_screen();
                print_header(" MARK AS MISSED ");
                
                let meds: Vec<_> = tracker.medications.keys().cloned().collect();
                if meds.is_empty() {
                    println!("No medications to mark.");
                    wait_for_enter();
                    continue;
                }
                
                for (i, med) in meds.iter().enumerate() {
                    println!("{}. {}", i + 1, med);
                }
                
                print!("Enter number: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                if let Ok(num) = input.trim().parse::<usize>() {
                    if num > 0 && num <= meds.len() {
                        let med_name = &meds[num - 1];
                        match tracker.mark_taken(med_name, &today, false) {
                            Ok(_) => println!("Recorded: {} missed", med_name),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        println!("Invalid selection.");
                    }
                }
                wait_for_enter();
            }
            
            "4" => {
                clear_screen();
                print_header(" ALL MEDICATIONS ");
                
                let meds = tracker.list_medications();
                if meds.is_empty() {
                    println!("No medications on record.");
                } else {
                    for med in meds {
                        println!("* {}", med);
                    }
                }
                wait_for_enter();
            }
            
            "5" => {
                clear_screen();
                print_header(" ADD NEW MEDICATION ");
                
                print!("Medication name: ");
                io::stdout().flush().unwrap();
                let mut name = String::new();
                io::stdin().read_line(&mut name).unwrap();
                
                print!("Dosage (e.g., '1 pill', '5ml'): ");
                io::stdout().flush().unwrap();
                let mut dosage = String::new();
                io::stdin().read_line(&mut dosage).unwrap();
                
                println!("Time of day:");
                println!("1. Morning");
                println!("2. Afternoon");
                println!("3. Evening");
                println!("4. Bedtime");
                print!("Select (1-4): ");
                io::stdout().flush().unwrap();
                let mut time_choice = String::new();
                io::stdin().read_line(&mut time_choice).unwrap();
                
                let time_of_day = match time_choice.trim() {
                    "1" => "Morning",
                    "2" => "Afternoon",
                    "3" => "Evening",
                    "4" => "Bedtime",
                    _ => "As needed",
                };
                
                print!("Starting quantity: ");
                io::stdout().flush().unwrap();
                let mut count = String::new();
                io::stdin().read_line(&mut count).unwrap();
                
                tracker.add_medication(
                    name.trim().to_string(),
                    dosage.trim().to_string(),
                    time_of_day.to_string(),
                    count.trim().parse().unwrap_or(30),
                );
                
                println!("Medication added!");
                wait_for_enter();
            }
            
            "6" => {
                clear_screen();
                print_header(" REFILL MEDICATION ");
                
                let meds: Vec<_> = tracker.medications.keys().cloned().collect();
                if meds.is_empty() {
                    println!("No medications to refill.");
                    wait_for_enter();
                    continue;
                }
                
                for (i, med) in meds.iter().enumerate() {
                    println!("{}. {}", i + 1, med);
                }
                
                print!("Enter number: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                
                if let Ok(num) = input.trim().parse::<usize>() {
                    if num > 0 && num <= meds.len() {
                        let med_name = &meds[num - 1];
                        
                        print!("Amount to add: ");
                        io::stdout().flush().unwrap();
                        let mut amount = String::new();
                        io::stdin().read_line(&mut amount).unwrap();
                        
                        match tracker.refill_medication(med_name, amount.trim().parse().unwrap_or(0)) {
                            Ok(_) => println!("{} refilled!", med_name),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        println!("Invalid selection.");
                    }
                }
                wait_for_enter();
            }
            
            "7" => {
                clear_screen();
                print_header(" WEEKLY SUMMARY ");
                
                let week_start = get_week_start();
                let summary = tracker.generate_weekly_summary(&week_start);
                println!("{}", summary);
                wait_for_enter();
            }
            
            "8" => {
                clear_screen();
                print_header(" SAVE WEEKLY REPORT ");
                
                let week_start = get_week_start();
                match tracker.save_chart_to_file(&week_start) {
                    Ok(filename) => println!("Report saved to: {}", filename),
                    Err(e) => println!("Error: {}", e),
                }
                wait_for_enter();
            }
            
            "9" => {
                clear_screen();
                println!("Goodbye!");
                break;
            }
            
            _ => {
                println!("Invalid choice.");
                wait_for_enter();
            }
        }
    }
}
