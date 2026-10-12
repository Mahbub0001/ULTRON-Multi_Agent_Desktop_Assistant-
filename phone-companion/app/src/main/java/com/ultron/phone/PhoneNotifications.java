package com.ultron.phone;

import android.app.Notification;
import android.app.RemoteInput;
import android.content.Intent;
import android.os.Bundle;
import android.service.notification.NotificationListenerService;
import android.service.notification.StatusBarNotification;
import org.json.JSONArray;
import org.json.JSONObject;

public class PhoneNotifications extends NotificationListenerService {
    static volatile PhoneNotifications instance;
    @Override public void onListenerConnected() { instance = this; }
    @Override public void onListenerDisconnected() { instance = null; }
    @Override public void onDestroy() { if (instance == this) instance = null; super.onDestroy(); }
    private Notification.Action replyAction(Notification notification) {
        if (notification.actions == null) return null;
        for (Notification.Action action : notification.actions) {
            if (action.actionIntent == null || action.getRemoteInputs() == null) continue;
            for (RemoteInput input : action.getRemoteInputs()) if (input.getAllowFreeFormInput()) return action;
        }
        return null;
    }
    JSONArray list() throws Exception {
        JSONArray result = new JSONArray();
        RankingMap rankings = getCurrentRanking();
        StatusBarNotification[] notifications = getActiveNotifications();
        if (notifications == null) return result;
        for (StatusBarNotification sbn : notifications) {
            if ((getPackageName().equals(sbn.getPackageName()) && sbn.getId() != NotificationTestReceiver.ID)
                || (sbn.getNotification().flags & Notification.FLAG_GROUP_SUMMARY) != 0) continue;
            Notification n = sbn.getNotification();
            Ranking ranking = new Ranking();
            int importance = rankings != null && rankings.getRanking(sbn.getKey(), ranking) ? ranking.getImportance() : 0;
            result.put(new JSONObject().put("key", sbn.getKey()).put("package", sbn.getPackageName())
                .put("title", safe(n.extras.getCharSequence(Notification.EXTRA_TITLE)))
                .put("text", safe(n.extras.getCharSequence(Notification.EXTRA_BIG_TEXT, n.extras.getCharSequence(Notification.EXTRA_TEXT))))
                .put("time", sbn.getPostTime()).put("importance", importance).put("can_reply", replyAction(n) != null));
            if (result.length() >= 100) break;
        }
        return result;
    }
    private String safe(CharSequence text) {
        String value = text == null ? "" : text.toString();
        return value.substring(0, Math.min(value.length(), 2000));
    }
    JSONObject reply(String key, String message) throws Exception {
        if (message.trim().isEmpty() || message.length() > 4000) throw new IllegalArgumentException("Reply must have 1 to 4000 characters.");
        StatusBarNotification[] active = getActiveNotifications();
        if (active != null) for (StatusBarNotification sbn : active) {
            if (!sbn.getKey().equals(key)) continue;
            Notification.Action action = replyAction(sbn.getNotification());
            if (action == null) throw new IllegalStateException("Notification does not support direct reply.");
            Bundle values = new Bundle();
            for (RemoteInput input : action.getRemoteInputs())
                if (input.getAllowFreeFormInput()) values.putCharSequence(input.getResultKey(), message);
            Intent intent = new Intent();
            RemoteInput.addResultsToIntent(action.getRemoteInputs(), intent, values);
            if (android.os.Build.VERSION.SDK_INT >= 28)
                RemoteInput.setResultsSource(intent, RemoteInput.SOURCE_FREE_FORM_INPUT);
            action.actionIntent.send(this, 0, intent);
            return new JSONObject().put("reply_dispatched", true).put("package", sbn.getPackageName());
        }
        throw new IllegalStateException("Notification is no longer active. Read notifications again.");
    }
}
