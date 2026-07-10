// Minimal C++ consumer of game_scanner_ffi.
//
// Build (Windows / MSVC):
//   cl /std:c++17 /EHsc /I ..\include examples\ffi_demo.cpp \
//      ..\target\release\game_scanner_ffi.lib
//
// Build (GCC / Clang):
//   g++ -std=c++17 -I../include examples/ffi_demo.cpp \
//       -L../target/release -lgame_scanner_ffi -o ffi_demo
//
// Copy `game_scanner_ffi.dll` (or .so/.dylib) next to the executable
// before running.

#include <cstdio>
#include <cstdlib>
#include <iostream>
#include <memory>
#include <string>

#include "game_scanner.h"

struct OwnedStr {
    char *p = nullptr;
    ~OwnedStr() { if (p) gs_free(p); }
    explicit operator bool() const { return p != nullptr; }
};

static int call_demo(const char *launcher) {
    OwnedStr body;
    int rc = gs_list(launcher, &body.p);
    if (rc != 0) {
        std::cerr << launcher << ": rc=" << rc
                  << " body=" << (body.p ? body.p : "(null)") << "\n";
        return rc;
    }
    // Tiny "JSON printer": just dump first 120 chars so the demo stays
    // dependency-free. Real code uses nlohmann/json or similar.
    std::string s(body.p ? body.p : "");
    if (s.size() > 120) s.resize(120);
    std::cout << launcher << ": " << s << "...\n";
    return 0;
}

int main() {
    call_demo("steam");
    call_demo("epicgames");
    return 0;
}
