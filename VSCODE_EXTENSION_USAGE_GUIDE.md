# 🎯 HipCortex VS Code Extension - Chat Interface Guide

## 🚀 Extension Successfully Deployed!

The HipCortex extension has been successfully installed in VS Code with chat participant integration.

---

## 📋 How to Use the Chat Interface

### 1. Open VS Code Chat
- Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (Mac)
- Type "Chat: Open Chat" and select it
- Or use the chat icon in the activity bar

### 2. Start HipCortex Chat
- In the chat interface, type `@hipcortex` to start interacting with HipCortex
- The chat participant will respond to your memory-related queries

---

## 💬 Available Chat Commands

### Memory Recording
```
@hipcortex add actor:Developer action:code_review target:user_authentication.py metadata:{"status":"approved","reviewer":"senior_dev"}
```

### Memory Queries
```
@hipcortex query actor:Developer limit:10
@hipcortex search action:code_review
@hipcortex find target:authentication
```

### System Status
```
@hipcortex health
@hipcortex status
```

### Help
```
@hipcortex help
```

---

## 🔧 Configuration

The extension uses these default settings:
- **API URL**: `http://127.0.0.1:3030`
- **Auto-start server**: `true`
- **API Key**: (none - optional)

To modify settings:
1. Go to `File > Preferences > Settings`
2. Search for "HipCortex"
3. Adjust the configuration as needed

---

## 🎮 Example Chat Interactions

### 1. Adding a Memory Record
**You:** `@hipcortex add actor:StudentDev action:learn_rust target:ownership_concepts`

**HipCortex:** ✅ **Memory Added Successfully!**
- **Record ID**: `abc123-def456-789`
- **Actor**: StudentDev
- **Action**: learn_rust
- **Target**: ownership_concepts
- **Timestamp**: 2025-09-13T11:40:35Z

### 2. Querying Memory
**You:** `@hipcortex query actor:StudentDev limit:5`

**HipCortex:** 📋 **Found 1 Memory Records**

| Actor | Action | Target | Timestamp |
|-------|--------|--------|-----------|
| StudentDev | learn_rust | ownership_concepts | 2025-09-13T11:40:35Z |

### 3. System Health Check
**You:** `@hipcortex health`

**HipCortex:** ✅ **HipCortex Server Status: Healthy**
- Server URL: http://127.0.0.1:3030
- Response time: 23ms
- Status: Connected

---

## 🔍 Advanced Features

### Metadata Support
```
@hipcortex add actor:DataScientist action:train_model target:customer_churn metadata:{"accuracy":0.94,"model_type":"random_forest","features":15}
```

### Complex Queries
```
@hipcortex query actor:DataScientist action:train_model limit:10
```

### Command Shortcuts
- `@hipcortex add ...` or `@hipcortex record ...`
- `@hipcortex query ...` or `@hipcortex search ...` or `@hipcortex find ...`

---

## 🚨 Troubleshooting

### Extension Not Responding
1. Check if HipCortex server is running:
   ```bash
   curl http://localhost:3030/health
   ```
2. Restart VS Code
3. Check VS Code Developer Console (`Help > Toggle Developer Tools`)

### Server Connection Issues
1. Verify server is running on port 3030
2. Check firewall settings
3. Ensure no other service is using port 3030

### Chat Participant Not Found
1. Reload VS Code window (`Ctrl+Shift+P` → "Reload Window")
2. Verify extension is enabled in Extensions panel
3. Check if chat feature is enabled in VS Code

---

## 🎯 Ready to Start!

Your HipCortex VS Code extension is now ready for use. You can:

1. **Open VS Code**
2. **Open the Chat interface** (`Ctrl+Shift+P` → "Chat: Open Chat")
3. **Start chatting** with `@hipcortex`
4. **Try the example commands** above

The extension will automatically connect to your local HipCortex server running on `http://127.0.0.1:3030` and provide intelligent memory management capabilities directly within your VS Code workflow!

---

*Extension Version: 0.1.0*  
*Server Status: ✅ Running on http://127.0.0.1:3030*  
*Ready for Memory Management! 🧠*
