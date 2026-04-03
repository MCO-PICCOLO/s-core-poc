#ifndef _PEER_DBUS_H_
#define _PEER_DBUS_H_

#ifdef __cplusplus
extern "C" {
#endif

#define LOG_ERROR(fmt, ...) \
	do { fprintf(stderr, "%s:%u: " fmt, __func__, __LINE__, ##__VA_ARGS__); } while(0)

#define LOG_INFO(fmt, ...) \
	do { fprintf(stdout, fmt, ##__VA_ARGS__); } while(0)

#define TRPC_SERVER_NAME	"com.lge.Timpani"
#define TRPC_SERVER_DESC	"Timpani-O"
#define TRPC_CLIENT_DESC	"Timpani-N"
#define TRPC_OBJECT_PATH	"/com/lge/Timpani"
#define TRPC_OBJECT_INTERFACE	"com.lge.Timpani.Orchestrator"
#define TRPC_METHOD_REGISTER	"Register"
#define TRPC_METHOD_SCHEDINFO	"SchedInfo"
#define TRPC_METHOD_DMISS	"DMiss"
#define TRPC_METHOD_SYNC	"Sync"

#ifdef __cplusplus
}
#endif

#endif	/* _PEER_DBUS_H_ */
