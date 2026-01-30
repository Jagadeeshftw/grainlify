
import os
import re

files = [
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_verify_program_schedule_tracking_and_history.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_single_program_release_schedule.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_program_overlapping_schedules.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_program_manual_trigger_before_after_timestamp.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_program_automatic_release_at_timestamp.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_multiple_program_release_schedules.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_lock_zero_funds.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_duplicate_program_registration.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_batch_payout_mismatched_lengths.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_batch_payout_insufficient_balance.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_anti_abuse_limit_panic.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/program-escrow/test_snapshots/test/test_anti_abuse_cooldown_panic.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/bounty_escrow/contracts/escrow/test_snapshots/test/test_refund_custom_before_deadline_without_approval.1.json",
    "/Users/winnergbolagade/Desktop/grainlify/contracts/bounty_escrow/contracts/escrow/test_snapshots/test/test_refund_approval_mismatch.1.json"
]

def resolve_file(filepath):
    try:
        with open(filepath, 'r') as f:
            content = f.read()
        
        # More robust pattern:
        # Match <<<<<<< HEAD ... ======= ... >>>>>>> ...
        # Use non-greedy matching for the content parts
        # Allow any characters in the marker lines
        pattern = re.compile(r'<<<<<<< HEAD.*?=======(.*?)>>>>>>>.*?(?:\n|$)', re.DOTALL)
        
        # Check if there are any matches
        if not pattern.search(content):
            print(f"No conflicts found matching pattern in {filepath}")
            # Check if there are markers at all
            if '<<<<<<<' in content:
                print(f"WARNING: Conflict markers found but regex failed in {filepath}")
            return

        new_content = pattern.sub(r'\1', content)
        
        if content != new_content:
            with open(filepath, 'w') as f:
                f.write(new_content)
            print(f"Resolved {filepath}")
        else:
            print(f"No changes made to {filepath}")
            
    except Exception as e:
        print(f"Error processing {filepath}: {e}")

for f in files:
    resolve_file(f)
