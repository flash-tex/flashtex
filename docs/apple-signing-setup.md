# Signing and notarizing FlashTeX (removing the Gatekeeper warning)

Today's releases are **ad-hoc signed**, so macOS refuses the first launch of a
browser-downloaded `FlashTeX.app` and sends people to System Settings ▸ Privacy &
Security ▸ "Open Anyway". This is not malware detection — Gatekeeper is refusing an
app with no verifiable developer identity.

`.github/workflows/release.yml` already signs with a Developer ID **and** notarizes
whenever the five secrets below exist, and falls back to ad-hoc when they do not.
Adding them is the whole fix; no code change is needed.

## What it costs

**Apple Developer Program — US$99/year.** There is no free route: Apple issues
Developer ID certificates only to paid members. (A free Apple ID gives you a
"Development" certificate, which runs on your own registered machines and does
*not* satisfy Gatekeeper for other people's downloads.)

## Steps

### 1. Enrol (one-off, 24-48h approval)
1. https://developer.apple.com/programs/enroll/ — sign in with the Apple ID you want
   to own the identity. Enrol as an **Individual** unless you want the app attributed
   to a company (an Organization needs a D-U-N-S number and takes longer).
2. Pay the $99 and wait for the approval email.
3. Note your **Team ID**: https://developer.apple.com/account → Membership details.
   It looks like `A1B2C3D4E5`.

### 2. Create the Developer ID Application certificate
On your Mac, with Xcode installed:
1. Xcode ▸ Settings ▸ Accounts ▸ add the Apple ID ▸ **Manage Certificates…**
2. **+** ▸ **Developer ID Application**. (Not "Apple Development" and not
   "Developer ID Installer" — the app bundle needs *Developer ID Application*.)
3. Keychain Access ▸ My Certificates ▸ find `Developer ID Application: <name> (<TeamID>)`.
4. Right-click it ▸ **Export…** ▸ `.p12` ▸ set a strong password. Exporting from
   *My Certificates* is what includes the private key; exporting from *Certificates*
   does not, and the CI import will fail.

### 3. Create an app-specific password for notarization
1. https://account.apple.com ▸ Sign-In and Security ▸ **App-Specific Passwords** ▸ **+**
2. Name it e.g. `flashtex-notary`, copy the `xxxx-xxxx-xxxx-xxxx` value.
   This is *not* your Apple ID password, and it is shown only once.

### 4. Add the five repository secrets
`base64` the certificate first:

```sh
base64 -i DeveloperID.p12 | pbcopy      # macOS
```

Then, in the repo (or via Settings ▸ Secrets and variables ▸ Actions):

```sh
gh secret set MAC_CERT_P12          # paste the base64 blob
gh secret set MAC_CERT_PASSWORD     # the .p12 export password
gh secret set NOTARY_APPLE_ID       # the Apple ID email
gh secret set NOTARY_TEAM_ID        # e.g. A1B2C3D4E5
gh secret set NOTARY_PASSWORD       # the app-specific password
```

### 5. Cut a release and check
The next tagged release signs, hardens the runtime, notarizes and staples the
ticket. Verify on a Mac that never built it:

```sh
spctl -a -vvv -t install /Applications/FlashTeX.app   # expect: accepted, source=Notarized Developer ID
xcrun stapler validate /Applications/FlashTeX.app     # expect: The validate action worked!
```

The release body stops saying "ad-hoc signed", and a browser download opens with a
normal "downloaded from the internet" prompt instead of the Privacy & Security detour.

## Notes

- **Keep the `.p12` and its password out of the repo.** GitHub secrets are the only
  place they belong; CI imports the certificate into a temporary keychain and deletes
  it afterwards.
- Developer ID certificates last 5 years. Note the expiry — a lapsed certificate
  fails the release job, it does not silently fall back.
- Notarization is a *service call*: the first one for a new account can take longer,
  and Apple occasionally rejects a build for a hardened-runtime or entitlement problem.
  The job surfaces `notarytool` output when that happens.
- Until this is set up, the curl installer on the download page avoids the warning
  entirely: it verifies the SHA-256 and installs without the browser quarantine flag.
