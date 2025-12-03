# Admin Panel Testing Guide

This document provides step-by-step instructions for testing the admin panel functionality.

## Prerequisites

1. Start the Rust backend server:
   ```bash
   cd rustpad-server
   cargo run
   ```

2. Start the frontend development server:
   ```bash
   npm run dev
   ```

3. Navigate to `http://localhost:5173`

## Test Scenarios

### 1. Admin User Registration

**Purpose**: Verify admin user can be created with proper privileges

**Steps**:
1. On the Rustpad home page, click "My Files" button in footer
2. Login modal appears - click "No account? Register here"
3. Fill in registration form:
   - Username: `admin_test`
   - Password: `SecurePass123!`
   - Check the "Admin" checkbox (⚠️ ADMIN WARNING should appear)
4. Click "Register"
5. Should see success message and modal closes

**Expected Results**:
- User registered successfully
- localStorage contains:
  - `rustpad_username`: `admin_test`
  - `rustpad_password`: `SecurePass123!`
  - `rustpad_is_admin`: `"true"`
  - `rustpad_ai_enabled`: `"false"` (default)

### 2. Admin Button Visibility

**Purpose**: Verify admin button only appears for admin users

**Steps**:
1. After admin registration, check footer
2. Should see "Admin" button with shield icon next to "My Files"
3. Logout (clear localStorage) and login as non-admin user
4. Admin button should disappear

**Expected Results**:
- Admin button visible only when `rustpad_is_admin === "true"`
- Button has shield icon (VscShield)
- Button has white text on blue background

### 3. Admin Panel Access Control

**Purpose**: Verify only admins can access admin panel

**Test 3a - Admin Access**:
1. Login as admin user (`admin_test`)
2. Click "Admin" button in footer
3. Admin panel modal opens

**Expected Results**:
- Modal opens with title "Admin Panel"
- Statistics cards visible
- User list table visible

**Test 3b - Non-Admin Access Denial**:
1. Create non-admin user via registration
2. Try clicking "Admin" button (should not be visible)
3. Manually set `rustpad_is_admin` to `"true"` in localStorage
4. Refresh page, click "Admin" button
5. Should see error toast: "Access denied - You do not have administrator privileges"

**Expected Results**:
- Backend validates admin status via Basic Auth
- Non-admin users get 403 Forbidden response
- Error message displayed to user

### 4. View User List

**Purpose**: Verify admin can see all registered users

**Steps**:
1. Login as admin
2. Click "Admin" button
3. Admin panel displays user list

**Expected Results**:
- Table shows columns: Username, Created, Roles, AI Access, Actions
- Admin user has "Admin" badge (red) and possibly "AI" badge (purple)
- Created dates formatted correctly
- Current user marked with "(You)"

### 5. Statistics Display

**Purpose**: Verify statistics cards show accurate counts

**Steps**:
1. Register multiple users (e.g., 3 regular users, 2 admins, enable AI for 1 user)
2. Open admin panel
3. Check statistics cards

**Expected Results**:
- "Total Users" shows correct count (5 in example)
- "AI Enabled" shows users with ai_enabled=true
- "Administrators" shows users with is_admin=true
- Numbers update in real-time when users added/removed

### 6. Toggle AI Access

**Purpose**: Verify admin can enable/disable AI for users

**Steps**:
1. Open admin panel
2. Find a user without AI access (Switch is OFF)
3. Click the AI Access switch to enable
4. Should see success toast: "AI access enabled for {username}"
5. Click again to disable
6. Should see success toast: "AI access disabled for {username}"

**Expected Results**:
- Switch toggles immediately in UI
- Backend API PUT `/api/admin/users/{username}/ai` succeeds
- User's AI status updates in database
- Statistics card "AI Enabled" updates
- User can now (or no longer) access AI features

### 7. Delete User

**Purpose**: Verify admin can delete users with confirmation

**Steps**:
1. Create a test user: `user_to_delete`
2. Open admin panel
3. Find `user_to_delete` in table
4. Click trash icon (VscTrash) in Actions column
5. Confirmation dialog appears
6. Click "Cancel" - dialog closes, user remains
7. Click trash icon again
8. Click "Delete" in confirmation dialog
9. Should see success toast: "{username} has been removed"

**Expected Results**:
- Confirmation dialog prevents accidental deletion
- User removed from table immediately
- Backend API DELETE `/api/admin/users/{username}` succeeds
- User file deleted from `{SAVE_DIR}/users/{username}.json`
- Statistics update to reflect removal

### 8. Self-Delete Protection

**Purpose**: Verify admin cannot delete their own account

**Steps**:
1. Login as `admin_test`
2. Open admin panel
3. Find your own username (should show "(You)")
4. Try clicking delete button - it should be DISABLED

**Alternative Test**:
1. If button is enabled, click it
2. Confirmation dialog appears
3. Click "Delete"
4. Should see warning toast: "Cannot delete yourself - You cannot delete your own admin account"

**Expected Results**:
- Delete button disabled for current user
- If somehow triggered, backend returns error
- Admin cannot accidentally lock themselves out

### 9. Refresh User List

**Purpose**: Verify refresh button updates user list

**Steps**:
1. Open admin panel
2. Note current user count
3. In another browser/incognito, register a new user
4. Return to admin panel, click refresh icon (VscRefresh)
5. User list reloads with new user

**Expected Results**:
- Refresh icon shows spinner during loading
- User list updates with latest data from backend
- Statistics recalculated

### 10. Dark Mode Support

**Purpose**: Verify admin panel respects dark mode theme

**Steps**:
1. In Rustpad editor, toggle dark mode (settings icon in sidebar)
2. Open admin panel
3. Check colors

**Expected Results**:
- Modal background: `#1e1e1e` (dark) or `white` (light)
- Text color: `#cbcaca` (dark) or default (light)
- Statistics cards: `#2d3748` (dark) or colored backgrounds (light)
- Table headers: `#888` (dark) or default (light)
- Borders: `#3c3c3c` (dark) or `gray.200` (light)

### 11. Admin-Only Backend Endpoints

**Purpose**: Verify backend properly restricts admin endpoints

**Test using curl or Postman**:

```bash
# Get admin credentials
USERNAME="admin_test"
PASSWORD="SecurePass123!"
AUTH=$(echo -n "$USERNAME:$PASSWORD" | base64)

# Test list users (should succeed for admin)
curl -H "Authorization: Basic $AUTH" http://localhost:7878/api/admin/users

# Test with non-admin user (should fail with 403)
curl -H "Authorization: Basic non_admin:password123" http://localhost:7878/api/admin/users

# Test toggle AI access
curl -X PUT -H "Authorization: Basic $AUTH" \
  -H "Content-Type: application/json" \
  -d '{"ai_enabled": true}' \
  http://localhost:7878/api/admin/users/test_user/ai

# Test delete user
curl -X DELETE -H "Authorization: Basic $AUTH" \
  http://localhost:7878/api/admin/users/test_user
```

**Expected Results**:
- Admin requests return 200 with data
- Non-admin requests return 403 Forbidden
- Malformed auth returns 401 Unauthorized

### 12. Edge Cases

**Test 12a - Empty User List**:
1. Delete all users except yourself
2. Open admin panel
3. Should show 1 user (you)

**Test 12b - No Admin Users**:
1. Remove all admins (except yourself)
2. "Administrators" stat should show 1

**Test 12c - All Users Have AI**:
1. Enable AI for all users
2. "AI Enabled" should equal "Total Users"

**Test 12d - Long Usernames**:
1. Register user with long name: `very_long_username_test_12345`
2. Check table formatting remains intact

**Test 12e - Special Characters in Username**:
1. Register user with special chars: `user+test@example`
2. Verify deletion works correctly (URL encoding)

## Troubleshooting

### Admin button not appearing
- Check localStorage: `rustpad_is_admin` should be `"true"`
- Clear localStorage and re-login
- Check console for errors

### Admin panel empty
- Verify backend is running on port 7878
- Check browser console for API errors
- Verify auth credentials in localStorage

### Cannot toggle AI access
- Check user exists in backend
- Verify admin has valid credentials
- Check network tab for API response

### Delete confirmation not appearing
- Check Chakra UI AlertDialog imports
- Verify `useDisclosure` hook working
- Check browser console for React errors

## Cleanup

After testing, clean up test data:
```bash
# Remove test user files
rm -rf ./frozen_documents/users/admin_test.json
rm -rf ./frozen_documents/users/user_to_delete.json
# ... etc
```

Or use the admin panel to delete test users.
