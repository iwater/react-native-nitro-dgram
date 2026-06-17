package com.margelo.nitro.udp;

import android.content.Context;
import android.net.wifi.WifiManager;
import android.util.Log;

public class UdpMulticastLockManager {
    private static final String TAG = "UdpMulticastLockMgr";
    private static WifiManager.MulticastLock multicastLock = null;
    private static int activeMembershipCount = 0;

    public static synchronized void init(Context context) {
        if (context == null) return;
        try {
            WifiManager wifiManager = (WifiManager) context.getApplicationContext().getSystemService(Context.WIFI_SERVICE);
            if (wifiManager != null && multicastLock == null) {
                multicastLock = wifiManager.createMulticastLock("NitroUdp::MulticastLock");
                multicastLock.setReferenceCounted(false);
                Log.d(TAG, "MulticastLock created");
            }
        } catch (Exception e) {
            Log.e(TAG, "Failed to create MulticastLock", e);
        }
    }

    public static synchronized void acquire() {
        activeMembershipCount++;
        Log.d(TAG, "acquire: active membership count = " + activeMembershipCount);
        if (multicastLock != null && !multicastLock.isHeld()) {
            try {
                multicastLock.acquire();
                Log.d(TAG, "MulticastLock acquired");
            } catch (Exception e) {
                Log.e(TAG, "Failed to acquire MulticastLock", e);
            }
        }
    }

    public static synchronized void release() {
        if (activeMembershipCount > 0) {
            activeMembershipCount--;
        }
        Log.d(TAG, "release: active membership count = " + activeMembershipCount);
        if (activeMembershipCount == 0 && multicastLock != null && multicastLock.isHeld()) {
            try {
                multicastLock.release();
                Log.d(TAG, "MulticastLock released");
            } catch (Exception e) {
                Log.e(TAG, "Failed to release MulticastLock", e);
            }
        }
    }
}
