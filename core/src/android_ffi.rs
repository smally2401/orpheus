use jni::objects::JClass;
use jni::objects::JObject;
use jni::EnvUnowned;
use jni::errors::ThrowRuntimeExAndDefault;
use jni::objects::Global;
use jni::sys::jboolean;
use std::sync::Once;
use std::sync::OnceLock;
use std::io::Cursor;

static INIT: Once = Once::new();
static CONTEXT_GLOBAL: OnceLock<Global<JObject<'static>>> = OnceLock::new();

const TEST_CLIP: &[u8] = include_bytes!("/home/iris/music/highway-61-revisited/like_a_rolling_stone.mp3");
static TEST_PLAYER: OnceLock<(rodio::MixerDeviceSink, rodio::Player)> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_orpheus_MainActivity_initAudioContext<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    context: JObject<'local>,
) -> jboolean {

    env.with_env(|env| -> jni::errors::Result<bool> {
        let vm = env.get_java_vm()?;
        let context_global = env.new_global_ref(&context)?;

        INIT.call_once(|| {
            unsafe {
                ndk_context::initialize_android_context(
                    vm.get_raw().cast(),
                    context_global.as_raw().cast(),
                );
            }

            let _ = CONTEXT_GLOBAL.set(context_global);
        });

        Ok(true)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_orpheus_MainActivity_playTestSound<'local>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
) -> jboolean {

    env.with_env(|_env| -> jni::errors::Result<bool> {
        let (_stream, player) = TEST_PLAYER.get_or_init(|| {
            let stream = rodio::DeviceSinkBuilder::open_default_sink()
                .expect("failed to open default audio sink");
            let player = rodio::Player::connect_new(stream.mixer());
            (stream, player)
        });

        let decoder = rodio::Decoder::new(Cursor::new(TEST_CLIP))
            .expect("test clip failed to decode");
        player.append(decoder);
        player.play();

        Ok(true)
    })
    .resolve::<ThrowRuntimeExAndDefault>()
}