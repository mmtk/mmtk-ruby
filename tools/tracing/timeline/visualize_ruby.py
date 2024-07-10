#!/usr/bin/env python3

def enrich_event_extra(log_processor, name, ph, tid, ts, result, args):
    match name:
        case "plan_end_of_gc":
            if ph == "B":
                result["args"] |= {
                    "vm": "Ruby",
                }

def enrich_meta_extra(log_processor, name, tid, ts, gc, wp, args):
    if wp is not None:
        match name:
            case "pin_ppps_prepare":
                young, old = int(args[0]), int(args[1])
                wp["args"] |= {
                    "pin_ppps_prepare": {
                        "young": young,
                        "old": old,
                        "total": young + old,
                    }
                }

            case "remove_dead_ppps":
                young, old, dead, no_longer = int(args[0]), int(args[1]), int(args[2]), int(args[3])
                total = young + old
                wp["args"] |= {
                    "young": young,
                    "old": old,
                    "total": total,
                    "removed_dead": dead,
                    "removed_no_longer": no_longer,
                }
            case "unpin_ppp_children":
                print("unpin!")
                wp["args"] |= {
                    "unpinned": int(args[0]),
                }

            case _:
                #print("Unrecognized meta {}".format(name))
                pass
