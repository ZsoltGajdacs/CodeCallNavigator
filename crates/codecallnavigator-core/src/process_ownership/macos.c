#include <errno.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/sysctl.h>

// Keep the SDK's kinfo_proc layout on the C side of the boundary.
struct process_row {
    uint64_t seconds;
    uint64_t microseconds;
    int32_t pid;
    int32_t parent;
    int32_t group;
};

static struct process_row read_row(const struct kinfo_proc *entry) {
    return (struct process_row){
        .seconds = entry->kp_proc.p_starttime.tv_sec,
        .microseconds = entry->kp_proc.p_starttime.tv_usec,
        .pid = entry->kp_proc.p_pid,
        .parent = entry->kp_eproc.e_ppid,
        .group = entry->kp_eproc.e_pgid,
    };
}

int ccn_process_observation(int32_t pid, struct process_row *output, size_t *count) {
    int mib[] = {CTL_KERN, KERN_PROC, KERN_PROC_PID, pid};
    struct kinfo_proc entry;
    size_t size = sizeof(entry);
    if (sysctl(mib, 4, &entry, &size, NULL, 0) == -1) {
        return errno;
    }
    if (size != 0 && size != sizeof(entry)) {
        return EPROTO;
    }
    *count = size != 0;
    if (*count) {
        *output = read_row(&entry);
    }
    return 0;
}

static int copy_table(const struct kinfo_proc *entries, size_t size,
                      struct process_row *output, size_t capacity, size_t *count) {
    if (size % sizeof(*entries) != 0) {
        return EPROTO;
    }
    *count = size / sizeof(*entries);
    if (*count > capacity) {
        return EOVERFLOW;
    }
    for (size_t i = 0; i < *count; i++) {
        output[i] = read_row(&entries[i]);
    }
    return 0;
}

int ccn_process_inventory(struct process_row *output, size_t capacity, size_t *count) {
    int mib[] = {CTL_KERN, KERN_PROC, KERN_PROC_ALL};
    size_t size = 0;
    if (sysctl(mib, 3, NULL, &size, NULL, 0) == -1) {
        return errno;
    }
    if (size == 0 || size > capacity * sizeof(struct kinfo_proc)) {
        return size == 0 ? EPROTO : EOVERFLOW;
    }
    const size_t allocated = size;
    struct kinfo_proc *entries = malloc(allocated);
    if (!entries) {
        return ENOMEM;
    }
    int status;
    if (sysctl(mib, 3, entries, &size, NULL, 0) == -1) {
        status = errno;
    } else if (size > allocated) {
        status = EPROTO;
    } else {
        status = copy_table(entries, size, output, capacity, count);
    }
    free(entries);
    return status;
}
