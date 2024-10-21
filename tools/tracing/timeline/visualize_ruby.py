#!/usr/bin/env python3

def enrich_meta_extra(log_processor, name, tid, ts, gc, wp, args):
    if wp is not None:
        match name:
            case "pin_ppp_children":
                num_ppps, num_no_longer_ppps, num_pinned_children = [int(x) for x in args]
                num_still_ppps = num_ppps - num_no_longer_ppps
                wp["args"] |= {
                    "num_ppps": {
                        "total": num_ppps,
                        "still_ppps": num_still_ppps,
                        "no_longer_ppps": num_no_longer_ppps,
                    },
                    "num_pinned_children": num_pinned_children,
                }

            case "remove_dead_ppps":
                num_ppps, num_no_longer_ppps, num_dead_ppps = [int(x) for x in args]
                num_retained_ppps = num_ppps - num_no_longer_ppps - num_dead_ppps
                wp["args"] |= {
                    "num_ppps": {
                        "total (before)": num_ppps,
                        "dead": num_dead_ppps,
                        "no_longer_ppps": num_no_longer_ppps,
                        "retained (after)": num_retained_ppps,
                    },
                    "num_retained_ppps": num_retained_ppps,
                }

            case "unpin_ppp_children":
                num_children = int(args[0])
                wp["args"] |= {
                    "num_ppp_children": num_children,
                }

            case "weak_table_size_change":
                before, after = [int(x) for x in args]
                wp["args"] |= {
                    "entries": {
                        "before": before,
                        "after": after,
                        "diff": after - before,
                    },
                }

            case "update_finalizer_and_obj_id_tables":
                (finalizer_before, finalizer_after,
                 obj_to_id_before, obj_to_id_after,
                 id_to_obj_before, id_to_obj_after) = [int(x) for x in args]
                wp["args"] |= {
                    "finalizer": { "before": finalizer_before, "after": finalizer_after, "diff": finalizer_after - finalizer_before },
                    "obj_to_id": { "before": obj_to_id_before, "after": obj_to_id_after, "diff": obj_to_id_after - obj_to_id_before },
                    "id_to_obj": { "before": id_to_obj_before, "after": id_to_obj_after, "diff": id_to_obj_after - id_to_obj_before },
                }

            case "initial_weak_table_stats":
                entries_start, entries_bound, bins_num, num_entries = [int(x) for x in args[0:4]]
                table_name = args[4]
                gc["args"].setdefault(table_name, {})
                gc["args"][table_name] |= {
                    "entries_start": entries_start,
                    "entries_bound": entries_bound,
                    "bins_num": bins_num,
                    "num_entries_before": num_entries,
                }

            case "final_weak_table_stats":
                num_entries = int(args[0])
                table_name = args[1]
                gc["args"].setdefault(table_name, {})
                gc["args"][table_name] |= {
                    "num_entries_after": num_entries,
                }
                if "num_entries_before" in gc["args"][table_name]:
                    before = gc["args"][table_name].pop("num_entries_before")
                    after = gc["args"][table_name].pop("num_entries_after")
                    gc["args"][table_name]["entries"] = {
                        "before": before,
                        "after": after,
                        "diff": after - before,
                    }

            case "update_table_entries_parallel":
                begin, end, deleted_entries = [int(x) for x in args[0:3]]
                table_name = args[3]
                num_entries = end - begin
                wp["args"] |= {
                    "begin": begin,
                    "end": end,
                    "num_entries": num_entries,
                    "deleted_entries": deleted_entries,
                    "table_name": table_name,
                }

            case "update_table_bins_parallel":
                begin, end, deleted_bins = [int(x) for x in args[0:3]]
                table_name = args[3]
                num_bins = end - begin
                wp["args"] |= {
                    "begin": begin,
                    "end": end,
                    "num_bins": num_bins,
                    "deleted_bins": deleted_bins,
                    "table_name": table_name,
                }

            case "update_generic_iv_tbl":
                entries_moved, old_entries, new_entries = [int(x) for x in args[0:3]]
                wp["args"] |= {
                    "entries_moved": entries_moved,
                    "entries": {
                        "before": old_entries,
                        "after": new_entries,
                        "diff": new_entries - old_entries,
                    },
                }

            case "process_obj_free_candidates":
                old_candidates, new_candidates = [int(x) for x in args[0:2]]
                wp["args"] |= {
                    "candidates": {
                        "before": old_candidates,
                        "after": new_candidates,
                        "diff": new_candidates - old_candidates,
                    },
                }
