/* Minimal MinGW-w64 runtime for linking the Rust windows-gnu build WITHOUT a MinGW-w64 install.
 *
 * rustup's x86_64-pc-windows-gnu std expects crt2.o + libmingw32/libmingwex/libgcc_eh from MinGW.
 * Where only clang and rust-lld exist (no x86_64-w64-mingw32-gcc), tools/win/setup.sh compiles this
 * file freestanding (no headers) and mingw-ld links it in instead:
 *   - the program entry points (console exe and DLL),
 *   - the TLS directory (_tls_used) that LLVM's native thread-locals need,
 *   - abort-only stubs for the libgcc_eh unwinder that std's panic_unwind references.
 * Consequence: a panic cannot be caught or unwound on this build; after the panic message is
 * printed the panicking thread simply ends (the whole process on the main thread, exit code 101).
 */
typedef unsigned long long u64;
typedef unsigned long u32; /* 32 bits on LLP64 (Windows) targets */

extern void abort(void);
extern u32 GetCurrentThreadId(void);
extern void ExitProcess(u32) __attribute__((noreturn));
extern void ExitThread(u32) __attribute__((noreturn));
u32 __shim_main_thread;

#ifdef SHIM_EXE
extern int main(int, char **, char **);
extern int __getmainargs(int *, char ***, char ***, int, void *); /* msvcrt.dll */
extern void exit(int);

void mainCRTStartup(void)
{
    int argc = 0, si = 0;
    char **argv = 0, **envp = 0;
    __shim_main_thread = GetCurrentThreadId();
    __getmainargs(&argc, &argv, &envp, 0, &si);
    exit(main(argc, argv, envp));
}
#else
int DllMainCRTStartup(void *h, u32 reason, void *reserved)
{
    (void)h; (void)reason; (void)reserved;
    return 1;
}
#endif

/* ---- C library bits msvcrt.dll (the Windows 7 era one) lacks; mingw's libmingwex normally has them ---- */
void __main(void) {} /* gcc's static-constructor hook: nothing to run for Rust */

extern double floor(double), ceil(double), sin(double), cos(double), sqrt(double);
extern float floorf(float), ceilf(float), sinf(float), cosf(float);
double trunc(double x) { return x < 0 ? ceil(x) : floor(x); }
float truncf(float x) { return x < 0 ? ceilf(x) : floorf(x); }
double round(double x)
{
    double t = trunc(x), d = x - t;
    if (d >= 0.5) t += 1.0; else if (d <= -0.5) t -= 1.0;
    return t;
}
float roundf(float x)
{
    float t = truncf(x), d = x - t;
    if (d >= 0.5f) t += 1.0f; else if (d <= -0.5f) t -= 1.0f;
    return t;
}
/* NOT fused: the product is rounded before the add (std's mul_add is only used for speed in the port) */
double fma(double x, double y, double z) { volatile double p = x * y; return p + z; }
float fmaf(float x, float y, float z) { volatile float p = x * y; return p + z; }
void sincos(double x, double *s, double *c) { *s = sin(x); *c = cos(x); }
void sincosf(float x, float *s, float *c) { *s = sinf(x); *c = cosf(x); }
float hypotf(float x, float y) { return (float)sqrt((double)x * x + (double)y * y); }

/* ---- TLS directory (what mingw's tlssup.c provides) ---- */
typedef void (*tls_cb)(void *, u32, void *);
typedef struct {
    u64 start, end, index, callbacks;
    u32 zero_fill, characteristics;
} tls_dir;

#define SEC(n) __attribute__((section(n), used))
SEC(".tls") char _tls_start = 0;
SEC(".tls$ZZZ") char _tls_end = 0;
SEC(".CRT$XLA") tls_cb __xl_a = 0;
SEC(".CRT$XLZ") tls_cb __xl_z = 0;
u32 _tls_index = 0;
SEC(".rdata$T") const tls_dir _tls_used = {
    (u64)&_tls_start, (u64)&_tls_end, (u64)&_tls_index, (u64)(&__xl_a + 1), 0, 0
};

/* ---- unwinder stubs (no libgcc_eh): reaching any of these means a panic or C++ exception ---- */
#define STUB0(n) int n(void) { abort(); return 0; }
STUB0(_Unwind_Resume)
STUB0(_Unwind_GetIP)
STUB0(_Unwind_GetIPInfo)
STUB0(_Unwind_GetLanguageSpecificData)
STUB0(_Unwind_GetRegionStart)
STUB0(_Unwind_GetDataRelBase)
STUB0(_Unwind_GetTextRelBase)
STUB0(_Unwind_SetGR)
STUB0(_Unwind_SetIP)
STUB0(_Unwind_DeleteException)
STUB0(_Unwind_Backtrace)
STUB0(_Unwind_FindEnclosingFunction)
/* Rust panics start here. There is no unwinder, so end the panicking thread (the panic message has
 * already been printed by the panic hook): ExitProcess(101) on the main thread, which is the exit code
 * of a Rust panic, ExitThread(101) otherwise (e.g. macroquad's audio thread panics when the machine
 * has no sound device; with a real unwinder only that thread would die too). */
int _Unwind_RaiseException(void *e)
{
    (void)e;
    if (GetCurrentThreadId() == __shim_main_thread) ExitProcess(101);
    ExitThread(101);
}
/* SEH personality: tell Windows "not handled, keep searching" (ExceptionContinueSearch = 1) */
int __gcc_personality_seh0(void) { return 1; }
int _GCC_specific_handler(void) { return 1; }
