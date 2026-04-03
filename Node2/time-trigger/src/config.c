#include "internal.h"
#include <unistd.h>

// Set default values
static void config_set_defaults(struct context *ctx)
{
    if (!ctx) {
        return;
    }

    // Explicit runtime state initialization
    ctx->runtime.shutdown_requested = 0;
    LIST_INIT(&ctx->runtime.workloads);
    ctx->runtime.nr_workloads = 0;
    LIST_INIT(&ctx->runtime.apex_list);

    ctx->config.cpu = -1;
    ctx->config.prio = -1;
    ctx->config.port = 7777;
    ctx->config.addr = "127.0.0.1";

    // Use hostname as default, fall back to "1" on failure
    if (gethostname(ctx->config.node_id, sizeof(ctx->config.node_id)) != 0) {
        strncpy(ctx->config.node_id, "1", sizeof(ctx->config.node_id) - 1);
    }
    ctx->config.node_id[sizeof(ctx->config.node_id) - 1] = '\0';
    ctx->config.enable_sync = false;
    ctx->config.enable_plot = false;
    ctx->config.enable_apex = false;
    ctx->config.clockid = CLOCK_REALTIME;
    ctx->config.log_level = TT_LOG_LEVEL_INFO;  // default log level
}

static void print_usage(const char *program_name)
{
    fprintf(stderr, "Usage: %s [options] [host]\n"
            "Options:\n"
            "  -c <cpu_num>\tcpu affinity for timetrigger\n"
            "  -P <prio>\tRT priority (1~99) for timetrigger\n"
            "  -p <port>\tport to connect to\n"
            "  -n <node id>\tNode ID (default: hostname)\n"
            "  -l <level>\tLog level (0=silent, 1=error, 2=warning, 3=info, 4=debug, 5=verbose)\n"
            "  -s\tEnable timer synchronization across multiple nodes\n"
            "  -g\tEnable saving plot data file by using BPF (<node id>.gpdata)\n"
            "  -a\tEnable Apex.OS test mode which works without TT schedule info\n"
            "  -h\tshow this help\n",
            program_name);
}

tt_error_t parse_config(int argc, char *argv[], struct context *ctx)
{
    config_set_defaults(ctx);

    int opt;
    while ((opt = getopt(argc, argv, "hc:P:p:n:l:sga")) >= 0) {
        switch (opt) {
        case 'c':
            ctx->config.cpu = atoi(optarg);
            break;
        case 'P':
            ctx->config.prio = atoi(optarg);
            break;
        case 'p':
            ctx->config.port = atoi(optarg);
            break;
        case 'n':
            strncpy(ctx->config.node_id, optarg, sizeof(ctx->config.node_id) - 1);
            ctx->config.node_id[sizeof(ctx->config.node_id) - 1] = '\0';
            break;
        case 'l':
            ctx->config.log_level = (tt_log_level_t)atoi(optarg);
            break;
        case 's':
            ctx->config.enable_sync = true;
            break;
        case 'g':
            ctx->config.enable_plot = true;
            break;
        case 'a':
            ctx->config.enable_apex = true;
            break;
        case 'h':
        default:
            print_usage(argv[0]);
            return TT_ERROR_CONFIG;
        }
    }

    if (optind < argc) {
        ctx->config.addr = argv[optind++];
    }

    return validate_config(ctx);
}

tt_error_t validate_config(const struct context *ctx)
{
    // Validate priority
    if (ctx->config.prio < -1 || ctx->config.prio > 99) {
        TT_LOG_ERROR("Invalid priority: %d (must be -1 or 1-99)", ctx->config.prio);
        return TT_ERROR_CONFIG;
    }

    // Validate port
    if (ctx->config.port <= 0 || ctx->config.port > 65535) {
        TT_LOG_ERROR("Invalid port: %d (must be 1-65535)", ctx->config.port);
        return TT_ERROR_CONFIG;
    }

    // Validate CPU (simple range check)
    if (ctx->config.cpu < -1 || ctx->config.cpu > 1024) {
        TT_LOG_ERROR("Invalid CPU number: %d", ctx->config.cpu);
        return TT_ERROR_CONFIG;
    }

    // Validate node ID
    if (strlen(ctx->config.node_id) == 0) {
        TT_LOG_ERROR("Node ID cannot be empty");
        return TT_ERROR_CONFIG;
    }

    // Validate and set log level
    if (ctx->config.log_level < TT_LOG_LEVEL_SILENT || ctx->config.log_level > TT_LOG_LEVEL_VERBOSE) {
        TT_LOG_ERROR("Invalid log level: %d (must be 0-5)", ctx->config.log_level);
        return TT_ERROR_CONFIG;
    }

    // Set global log level
    tt_set_log_level(ctx->config.log_level);

    TT_LOG_INFO("Configuration:");
    TT_LOG_INFO("  CPU affinity: %d", ctx->config.cpu);
    TT_LOG_INFO("  Priority: %d", ctx->config.prio);
    TT_LOG_INFO("  Server: %s:%d", ctx->config.addr, ctx->config.port);
    TT_LOG_INFO("  Node ID: %s", ctx->config.node_id);
    TT_LOG_INFO("  Log level: %d", ctx->config.log_level);
    TT_LOG_INFO("  Sync enabled: %s", ctx->config.enable_sync ? "yes" : "no");
    TT_LOG_INFO("  Plot enabled: %s", ctx->config.enable_plot ? "yes" : "no");
    TT_LOG_INFO("  Apex.OS test mode: %s", ctx->config.enable_apex ? "yes" : "no");

    return TT_SUCCESS;
}
