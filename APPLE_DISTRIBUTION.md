# Apple Distribution & Compliance Guide

## Current Status: ✅ BETA Distribution Ready (Ad-Hoc Signed)

Your app is currently configured for **beta/testing distribution** using ad-hoc signing. This is **FINE** for:

- Personal use
- Direct distribution to testers
- GitHub Releases
- Beta testing programs

## What You Have Now

### ✅ Already Configured

1. **Ad-hoc Signing** (`signingIdentity: "-"`)

   - Uses local Mac's signing certificate
   - Works for manual distribution
   - Users need to right-click → Open first time

2. **Proper Bundle Identifier**: `com.beneccles.crossover-mod-manager`

   - Follows Apple's reverse-domain naming
   - Unique identifier for your app

3. **Minimum macOS Version**: 10.15 (Catalina)

   - Should update to 11.0 since you're Apple Silicon only

4. **Valid Icons**: ICNS format included

   - Required for macOS apps

5. **DMG Distribution**: Already configured
   - Professional looking installer

## What You DON'T Need Yet (But Will Eventually)

### For Current BETA Distribution

**You DON'T need these yet:**

- ❌ Apple Developer Account ($99/year)
- ❌ Code signing certificate
- ❌ Notarization
- ❌ App Store submission

### User Experience with Ad-Hoc Signed App

When users download your BETA:

1. **Download DMG** from GitHub Releases
2. **Open DMG** and drag to Applications
3. **First Launch**: macOS will block it (Gatekeeper)
4. **Workaround**: Right-click → Open → Click "Open" again
5. **Subsequent Launches**: Opens normally

⚠️ **Users will see**: "Crossover Mod Manager cannot be opened because it is from an unidentified developer"

This is **NORMAL** for beta software distributed outside the App Store.

## When You'll Need Apple Developer Account

You'll need to upgrade when you want:

### 1. **Wider Distribution** (Remove Gatekeeper warnings)

- Users can double-click to open without security warning
- More professional appearance
- Required for apps distributed to general public

### 2. **App Store Distribution**

- Reach wider audience
- Automatic updates through App Store
- Built-in payment processing (if going paid)

### 3. **TestFlight Distribution**

- Apple's official beta testing platform
- Up to 10,000 external testers
- Automatic crash reporting

## How to Upgrade (When Ready)

### Cost: $99/year for Apple Developer Program

### Steps to Enable Proper Signing & Notarization

1. **Join Apple Developer Program**

   - Go to: https://developer.apple.com/programs/
   - Sign up with your Apple ID
   - Pay $99/year fee

2. **Create Certificates** (in Xcode or online)

   ```bash
   # Developer ID Application Certificate
   # Used for apps distributed outside the App Store
   ```

3. **Update tauri.conf.json**

   ```json
   "macOS": {
     "frameworks": [],
     "minimumSystemVersion": "11.0",  // Update for Apple Silicon
     "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)",
     "entitlements": null,
     "exceptionDomain": "",
     "provisioningProfile": null
   }
   ```

4. **Add Notarization to CI/CD**

   Update `.github/workflows/release.yml`:

   ```yaml
   - name: Notarize app
     env:
       APPLE_ID: ${{ secrets.APPLE_ID }}
       APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}
       APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
     run: |
       # Notarization commands here
       xcrun notarytool submit "Crossover Mod Manager.app" \
         --apple-id "$APPLE_ID" \
         --password "$APPLE_PASSWORD" \
         --team-id "$APPLE_TEAM_ID" \
         --wait
   ```

5. **Add Secrets to GitHub**
   - `APPLE_ID`: Your Apple ID email
   - `APPLE_PASSWORD`: App-specific password (not your Apple ID password)
   - `APPLE_TEAM_ID`: Your team ID from Apple Developer portal

## Recommended Timeline

### Phase 1: BETA (Current) ✅

**Duration**: 1-3 months

- Use ad-hoc signing
- Distribute via GitHub Releases
- Collect feedback from testers
- Fix bugs and improve stability

**Requirements**:

- ✅ Current setup is sufficient
- ✅ No Apple Developer account needed

### Phase 2: Public Beta

**Duration**: 1-2 months

- Still using ad-hoc signing OR
- Get Developer account for better UX
- Wider distribution to community
- Marketing and promotion

**Requirements**:

- Consider getting Apple Developer account
- Improves user experience
- Optional but recommended

### Phase 3: Stable Release (v1.0.0)

**Duration**: Ongoing

- **Definitely need** Apple Developer account
- Proper code signing
- Notarization
- Professional distribution

**Requirements**:

- ✅ Apple Developer account ($99/year)
- ✅ Code signing certificate
- ✅ Notarization process

## Current Recommendations

### For v0.1.0-beta1 (Your First Release)

**You're good to go!** 🎉

✅ **DO**:

- Release as-is with ad-hoc signing
- Include clear installation instructions in release notes
- Explain the "unidentified developer" warning
- Tell users to right-click → Open

✅ **INCLUDE IN RELEASE NOTES**:

```markdown
## ⚠️ Installation Note

macOS will show a security warning when opening this app.

**How to install:**

1. Download and open the DMG
2. Drag app to Applications folder
3. **First time**: Right-click the app → Select "Open" → Click "Open" again
4. Subsequent launches work normally

This is expected for beta software. A fully signed version will be
available in future releases.
```

❌ **DON'T**:

- Pay for Apple Developer yet
- Worry about notarization
- Stress about Gatekeeper warnings

### Quick Wins (No Cost)

1. **Update Minimum macOS Version**

   ```json
   "minimumSystemVersion": "11.0"
   ```

   (Since you're Apple Silicon only anyway)

2. **Add Helpful Documentation**

   - Security warnings are normal
   - Clear installation instructions
   - Video or screenshots showing right-click process

3. **Add to README.md**
   ```markdown
   ## Known Issues

   - First launch requires right-click → Open due to lack of Apple notarization
   - This is normal for beta software
   - Fully signed version coming in future releases
   ```

## Legal Considerations

### What You're Distributing

- ✅ Free software
- ✅ Open source (MIT License)
- ✅ Personal/hobby project

### What You're NOT Doing (Yet)

- ❌ Charging money
- ❌ App Store distribution
- ❌ Claiming official Apple endorsement

### Legal Status: ✅ OK to Distribute

You CAN legally distribute unsigned apps on macOS as long as:

- ✅ Users know it's unsigned (they'll see the warning)
- ✅ You're not malicious (obviously you're not)
- ✅ You're not misrepresenting the app
- ✅ Users explicitly choose to open it (right-click → Open)

## Summary: What Do You Need NOW?

### For v0.1.0-beta1: **NOTHING!** ✅

Your current setup is:

- ✅ Legal
- ✅ Functional
- ✅ Appropriate for beta testing
- ✅ Compliant with Apple's policies

### Action Items (Immediate):

1. ✅ Update minimum macOS to 11.0
2. ✅ Add installation instructions to release notes
3. ✅ Document security warning in README
4. ✅ Release your beta!

### Action Items (Future):

1. ⏳ **After 10-20 beta testers**: Consider Apple Developer account
2. ⏳ **Before v1.0.0 stable**: Get Developer account for sure
3. ⏳ **Long term**: App Store submission (optional)

## Questions?

- **"Will my app work?"** → YES
- **"Is it legal?"** → YES
- **"Will users complain?"** → They'll see a security warning (normal for beta)
- **"Should I pay $99 now?"** → NO, wait until you have beta feedback
- **"When do I need it?"** → Before v1.0.0 stable release

## Resources

- [Apple Developer Program](https://developer.apple.com/programs/)
- [Tauri Signing Guide](https://tauri.app/v1/guides/distribution/sign-macos)
- [Apple Notarization Guide](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)

---

**Bottom Line**: Ship your beta now, upgrade to proper signing later. Your current setup is perfect for beta testing! 🚀
