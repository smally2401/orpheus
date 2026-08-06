package com.orpheus

import android.content.Context
import android.os.Bundle
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Column
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text

class MainActivity : ComponentActivity() {
    external fun initAudioContext(context: Context): Boolean
    external fun playTestSound(): Boolean

    companion object {
        init {
            System.loadLibrary("orpheus_core")
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val ok = initAudioContext(applicationContext)
        Log.d("Orpheus", "initAudioContext returned: $ok")

        setContent {
            MaterialTheme {
                Column {
                    Text(text = "hello world!")
                    Button(onClick = {
                        val ok = playTestSound()
                        Log.d("Orpheus", "playTestSound returned: $ok")
                    }) {
                        Text("Play test sound")
                    }
                }
            }
        }
    }
}