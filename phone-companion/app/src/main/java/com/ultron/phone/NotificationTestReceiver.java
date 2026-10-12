package com.ultron.phone;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.RemoteInput;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.os.Bundle;

/** An entirely local direct-reply diagnostic; never contacts another person. */
public class NotificationTestReceiver extends BroadcastReceiver {
    static final int ID = 90210;
    static volatile String lastReply = "";
    static void post(Context context) {
        lastReply = "";
        NotificationManager manager = context.getSystemService(NotificationManager.class);
        if (!manager.areNotificationsEnabled()) throw new IllegalStateException("Notification permission is not enabled.");
        manager.createNotificationChannel(new NotificationChannel("diagnostic", "Local connection test", NotificationManager.IMPORTANCE_DEFAULT));
        PendingIntent reply = PendingIntent.getBroadcast(context, 0, new Intent(context, NotificationTestReceiver.class),
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_MUTABLE);
        RemoteInput input = new RemoteInput.Builder("test_reply").setLabel("Local test reply").build();
        Notification.Action action = new Notification.Action.Builder(null, "Reply locally", reply).addRemoteInput(input).build();
        manager.notify(ID, new Notification.Builder(context, "diagnostic").setContentTitle("ULTRON local notification test")
            .setContentText("Reply stays on this phone; no contact is messaged.")
            .setSmallIcon(android.R.drawable.stat_notify_chat).addAction(action).build());
    }
    @Override public void onReceive(Context context, Intent intent) {
        Bundle results = RemoteInput.getResultsFromIntent(intent);
        if (results != null) {
            lastReply = String.valueOf(results.getCharSequence("test_reply", ""));
            context.getSystemService(NotificationManager.class).cancel(ID);
        }
    }
}
