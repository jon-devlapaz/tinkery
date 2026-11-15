#include <errno.h>
#include <stdarg.h>
#include <stdint.h>
#include <fcntl.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/* Affect only the explicitly named TEST directory, while the fault flag exists. */
static int fail_sync(int fd) {
    const char *target = getenv("TINKERY_TEST_SYNC_DIRECTORY");
    const char *flag = getenv("TINKERY_TEST_SYNC_FLAG");
    const char *log = getenv("TINKERY_TEST_SYNC_LOG");
    struct stat status;
    char path[4096];
    if (target && strstr(target, "TEST") && flag && log &&
        fstat(fd, &status) == 0 && S_ISDIR(status.st_mode) &&
        fcntl(fd, F_GETPATH, path) == 0 && strcmp(path, target) == 0 &&
        access(flag, F_OK) == 0) {
        int out = open(log, O_WRONLY | O_CREAT | O_APPEND, 0600);
        if (out >= 0) { write(out, "directory sync: injected EIO\n", sizeof("directory sync: injected EIO\n") - 1); close(out); }
        errno = EIO;
        return -1;
    }
    return 0;
}
static int test_fsync(int fd) {
    if (fail_sync(fd)) return -1;
    return fsync(fd);
}
static int test_fcntl(int fd, int command, ...) {
    if (command == F_FULLFSYNC || command == F_BARRIERFSYNC) {
        if (fail_sync(fd)) return -1;
        return fcntl(fd, command);
    }
    if (command == F_GETFD || command == F_GETFL || command == F_GETOWN)
        return fcntl(fd, command);
    va_list args;
    va_start(args, command);
    uintptr_t arg = va_arg(args, uintptr_t);
    va_end(args);
    return fcntl(fd, command, arg);
}
__attribute__((used)) static struct { const void *replacement; const void *original; }
interposers[] __attribute__((section("__DATA,__interpose"))) = {
    {(const void *)test_fsync, (const void *)fsync},
    {(const void *)test_fcntl, (const void *)fcntl}
};
