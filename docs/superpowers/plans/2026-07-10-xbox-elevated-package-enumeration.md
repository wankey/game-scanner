# Xbox Elevated Package Enumeration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enumerate Xbox packages for all Windows users only when the scanner process has an elevated token, while preserving non-elevated current-user scanning.

**Architecture:** Keep package selection in `src/xbox/platform/windows.rs`. A small pure `PackageScope` decision function makes the elevation branch testable. A Win32 token query supplies the boolean used by that function; the selected WinRT API returns the same package iterable consumed by the existing filter and manifest pipeline.

**Tech Stack:** Rust 2021, `windows` 0.61 WinRT projection, Win32 token APIs, Cargo unit tests.

## Global Constraints

- Use `FindPackages()` only for a process whose token reports `TokenIsElevated != 0`.
- Use `FindPackagesByUserSecurityId(&HSTRING::new())` for a non-elevated process; the empty SID targets the current user.
- Do not change package filtering, launch, uninstall, manifest parsing, or public Xbox APIs.
- Surface token-query and enumeration failures as `ErrorKind::IO` with the relevant API name.

---

### Task 1: Add Required Windows API Features

**Files:**
- Modify: `Cargo.toml:47-51`
- Modify: `Cargo.lock` through Cargo resolution only if Cargo changes it

**Interfaces:**
- Consumes: the existing Windows-only `windows = { version = "0.61", ... }` dependency.
- Produces: access to `CloseHandle`, `GetTokenInformation`, `TOKEN_ELEVATION`, `TOKEN_QUERY`, `GetCurrentProcess`, and `OpenProcessToken`.

- [ ] **Step 1: Add the Win32 feature flags**

Extend the existing feature list without changing the dependency version:

```toml
windows = { version = "0.61", features = [
    "Management_Deployment",
    "ApplicationModel",
    "Win32_Foundation",
    "Win32_Security",
    "Win32_System_Threading",
] }
```

- [ ] **Step 2: Check the Windows dependency compiles**

Run: `cargo check -p game-scanner`

Expected: exits with status `0`; no unresolved `windows::Win32` imports remain.

- [ ] **Step 3: Commit the dependency feature change**

```powershell
git add -- Cargo.toml Cargo.lock
git commit -m "build(xbox): enable token elevation APIs"
```

### Task 2: Select Enumeration Scope From Elevation State

**Files:**
- Modify: `src/xbox/platform/windows.rs:1-68`
- Test: `src/xbox/platform/windows.rs:149-184`

**Interfaces:**
- Consumes: a boolean from `current_process_is_elevated()`.
- Produces: `PackageScope::{AllUsers, CurrentUser}` through `package_scope(is_elevated: bool) -> PackageScope`.

- [ ] **Step 1: Write failing tests for the scope decision**

Add these tests to the existing `tests` module:

```rust
#[test]
fn package_scope_uses_all_users_for_an_elevated_process() {
    assert_eq!(package_scope(true), PackageScope::AllUsers);
}

#[test]
fn package_scope_uses_current_user_for_a_non_elevated_process() {
    assert_eq!(package_scope(false), PackageScope::CurrentUser);
}
```

- [ ] **Step 2: Verify the tests fail for the missing behavior**

Run: `cargo test -p game-scanner xbox::platform::windows::tests::package_scope -- --nocapture`

Expected: FAIL because `package_scope` and `PackageScope` are undefined.

- [ ] **Step 3: Add the minimum scope decision implementation**

Place this directly above `get_packages`:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageScope {
    AllUsers,
    CurrentUser,
}

fn package_scope(is_elevated: bool) -> PackageScope {
    if is_elevated {
        PackageScope::AllUsers
    } else {
        PackageScope::CurrentUser
    }
}
```

- [ ] **Step 4: Verify the scope decision tests pass**

Run: `cargo test -p game-scanner xbox::platform::windows::tests::package_scope -- --nocapture`

Expected: both `package_scope_*` tests pass.

- [ ] **Step 5: Commit the tested decision function**

```powershell
git add -- src/xbox/platform/windows.rs
git commit -m "test(xbox): cover package enumeration scope"
```

### Task 3: Query the Process Token and Use the Selected WinRT API

**Files:**
- Modify: `src/xbox/platform/windows.rs:1-85`
- Test: `src/xbox/platform/windows.rs:149-196`

**Interfaces:**
- Consumes: `package_scope(bool) -> PackageScope` from Task 2 and the Win32 feature flags from Task 1.
- Produces: `current_process_is_elevated() -> Result<bool>` and a `get_packages()` path that returns all-user packages only for elevated processes.

- [ ] **Step 1: Add the Win32 imports**

Replace the current imports with these additions, retaining the existing error and deployment imports:

```rust
use crate::error::{Error, ErrorKind, Result};
use std::{mem::size_of, path::PathBuf};
use windows::{
    core::HSTRING,
    Management::Deployment::PackageManager,
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
};
```

- [ ] **Step 2: Implement the elevation query with deterministic handle cleanup**

Add this directly below `package_scope`:

```rust
fn current_process_is_elevated() -> Result<bool> {
    let mut token = HANDLE::default();
    unsafe {
        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).map_err(|e| {
            Error::new(ErrorKind::IO, format!("OpenProcessToken failed: {e}"))
        })?;

        let mut elevation = TOKEN_ELEVATION::default();
        let query_result = GetTokenInformation(
            token,
            TokenElevation,
            Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut 0,
        );
        let close_result = CloseHandle(token);

        query_result.map_err(|e| {
            Error::new(ErrorKind::IO, format!("GetTokenInformation(TokenElevation) failed: {e}"))
        })?;
        close_result.map_err(|e| Error::new(ErrorKind::IO, format!("CloseHandle failed: {e}")))?;

        Ok(elevation.TokenIsElevated != 0)
    }
}
```

- [ ] **Step 3: Replace the unconditional enumeration**

Replace the current `pm.FindPackages()` call in `get_packages` with:

```rust
let (packages, operation) = match package_scope(current_process_is_elevated()?) {
    PackageScope::AllUsers => (pm.FindPackages(), "FindPackages"),
    PackageScope::CurrentUser => {
        let current_user = HSTRING::new();
        (
            pm.FindPackagesByUserSecurityId(&current_user),
            "FindPackagesByUserSecurityId",
        )
    }
};
let packages = packages
    .map_err(|e| Error::new(ErrorKind::IO, format!("{operation} failed: {e}")))?;
```

Update the nearby comment to state that all-user enumeration is selected only for an elevated token, and that the empty SID selects the current user otherwise.

- [ ] **Step 4: Run the targeted Xbox tests**

Run: `cargo test -p game-scanner xbox::platform::windows::tests -- --nocapture`

Expected: all existing Xbox platform tests and both `package_scope_*` tests pass.

- [ ] **Step 5: Run the workspace suite**

Run: `cargo test --workspace`

Expected: exits with status `0`.

- [ ] **Step 6: Commit the elevation-aware enumeration**

```powershell
git add -- src/xbox/platform/windows.rs
git commit -m "fix(xbox): scope package scans to token elevation"
```

## Plan Self-Review

- Spec coverage: Task 1 enables the native APIs; Task 2 tests deterministic selection; Task 3 queries the actual process token, chooses the correct WinRT method, preserves `ErrorKind::IO`, closes the token handle, and verifies targeted plus workspace tests.
- Placeholder scan: no deferred or unspecified implementation steps remain.
- Type consistency: `PackageScope` and `package_scope(bool)` are defined in Task 2 and consumed with the same signatures in Task 3. Both WinRT calls return `IIterable<Package>`, allowing one shared filtering loop.

