#!/usr/bin/env python3

def enrich_meta_extra(log_processor, name, tid, ts, gc, wp, args):
    if wp is not None:
        match name:
            # PPPs

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

            # Generic weak table processing

            case "weak_table_size_change":
                before, after = [int(x) for x in args]
                wp["args"] |= {
                    "entries": {
                        "before": before,
                        "after": after,
                        "diff": after - before,
                    },
                }

            # Specific weak table processing work packets

            case "update_finalizer_and_obj_id_tables":
                (finalizer_before, finalizer_after,
                 id2ref_before, id2ref_after) = [int(x) for x in args]
                wp["args"] |= {
                    "finalizer": { "before": finalizer_before, "after": finalizer_after, "diff": finalizer_after - finalizer_before },
                    "id2ref": { "before": id2ref_before, "after": id2ref_after, "diff": id2ref_after - id2ref_before },
                }

            # Weak concurrent set optimization

            case "weak_cs_par_init":
                num_entries, capacity = [int(x) for x in args[0:2]]
                set_name = args[2]
                gc["args"].setdefault(set_name, {})
                gc["args"][set_name] |= {
                    "num_entries_before": num_entries,
                    "capacity": capacity,
                }

            case "weak_cs_par_final":
                num_entries = int(args[0])
                set_name = wp["args"]["set_name"]
                gc["args"].setdefault(set_name, {})
                gc["args"][set_name] |= {
                    "num_entries_after": num_entries,
                }
                if "num_entries_before" in gc["args"][set_name]:
                    before = gc["args"][set_name].pop("num_entries_before")
                    after = gc["args"][set_name].pop("num_entries_after")
                    gc["args"][set_name]["entries"] = {
                        "before": before,
                        "after": after,
                        "diff": after - before,
                    }

            case "weak_cs_par_entries_begin":
                begin, end = [int(x) for x in args[0:2]]
                set_name = args[-1]
                num_entries = end - begin
                wp["args"] |= {
                    "begin": begin,
                    "end": end,
                    "num_entries": num_entries,
                    "set_name": set_name,
                }

            case "weak_cs_par_entries_end":
                live, moved, deleted = [int(x) for x in args[0:3]]
                wp["args"] |= {
                    "live": live,
                    "moved": moved,
                    "deleted": deleted,
                }

            # Weak st table optimization

            case "weak_st_par_init":
                entries_start, entries_bound, bins_num, num_entries = [int(x) for x in args[0:4]]
                table_name = args[4]
                gc["args"].setdefault(table_name, {})
                gc["args"][table_name] |= {
                    "entries_start": entries_start,
                    "entries_bound": entries_bound,
                    "bins_num": bins_num,
                    "num_entries_before": num_entries,
                }

            case "weak_st_par_final":
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

            case "weak_st_par_entries":
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

            case "weak_st_par_bins":
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

            # Other work packets

            case "process_obj_free_candidates":
                old_candidates, new_candidates = [int(x) for x in args[0:2]]
                wp["args"] |= {
                    "candidates": {
                        "before": old_candidates,
                        "after": new_candidates,
                        "diff": new_candidates - old_candidates,
                    },
                }

            case "update_wb_unprotected_objects_list":
                before, after = [int(x) for x in args[0:2]]
                wp["args"] |= {
                    "wb_unprotected_objects": {
                        "before": before,
                        "after": after,
                        "diff": after - before,
                    },
                }
