#!/usr/bin/env python3

def enrich_meta_extra(log_processor, name, tid, ts, gc, wp, args):
    if wp is not None:
        match name:
            case "pin_ppp_children":
                num_ppps, num_no_longer_ppps, num_pinned_children = [int(x) for x in args]
                num_still_ppps = num_ppps - num_no_longer_ppps
                wp["args"] |= {
                    "num_ppps": num_ppps,
                    "num_no_longer_ppps": num_no_longer_ppps,
                    "num_still_ppps": num_still_ppps,
                    "num_pinned_children": num_pinned_children,
                }

            case "remove_dead_ppps":
                num_ppps, num_no_longer_ppps, num_dead_ppps = [int(x) for x in args]
                num_retained_ppps = num_ppps - num_no_longer_ppps - num_dead_ppps
                wp["args"] |= {
                    "num_ppps": num_ppps,
                    "num_no_longer_ppps": num_no_longer_ppps,
                    "num_dead_ppps": num_dead_ppps,
                    "num_retained_ppps": num_retained_ppps,
                }

            case "unpin_ppp_children":
                num_children = int(args[0])
                wp["args"] |= {
                    "num_ppp_children": num_children,
                }

            case "weak_table_size_change":
                old_entries, new_entries = [int(x) for x in args]
                wp["args"] |= {
                    "old_entries": old_entries,
                    "new_entries": new_entries,
                }

            case "update_finalizer_and_obj_id_tables":
                (old_finalizer, new_finalizer,
                 old_obj_to_id, new_obj_to_id,
                 old_id_to_obj, new_id_to_obj) = [int(x) for x in args]
                wp["args"] |= {
                    "finalizer": { "old_entries": old_finalizer, "new_entries": new_finalizer, },
                    "obj_to_id": { "old_entries": old_obj_to_id, "new_entries": new_obj_to_id, },
                    "id_to_obj": { "old_entries": old_id_to_obj, "new_entries": new_id_to_obj, },
                }
