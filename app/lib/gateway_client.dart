import 'dart:async';
import 'dart:convert';

import 'package:web_socket_channel/web_socket_channel.dart';
import 'package:web_socket_channel/status.dart' as ws_status;

/// A thin client over the dash gateway WebSocket.
///
/// It speaks the same JSON protocol the Rust `dash-gateway` crate defines:
///
/// - **Outgoing** [ClientCommand]s: `voice`, `set_destination`, `set_setting`.
/// - **Incoming** server events: flattened bus events with a `type` field
///   (`media_state`, `nav_destination`, `setting_changed`).
///
/// Incoming frames are decoded to `Map<String, dynamic>` and exposed on
/// [events]; the UI switches on the `"type"` key.
class GatewayClient {
  GatewayClient(this.url);

  /// The gateway endpoint, e.g. `ws://127.0.0.1:8080/ws`.
  final Uri url;

  WebSocketChannel? _channel;
  final _events = StreamController<Map<String, dynamic>>.broadcast();
  final _connected = StreamController<bool>.broadcast();

  /// Decoded server events. Broadcast, so multiple widgets can listen.
  Stream<Map<String, dynamic>> get events => _events.stream;

  /// Connection status changes: `true` when connected, `false` on drop.
  Stream<bool> get connected => _connected.stream;

  /// Open the connection and begin forwarding decoded frames to [events].
  void connect() {
    final channel = WebSocketChannel.connect(url);
    _channel = channel;
    _connected.add(true);
    channel.stream.listen(
      (data) {
        try {
          final decoded = jsonDecode(data as String);
          if (decoded is Map<String, dynamic>) {
            _events.add(decoded);
          }
        } catch (_) {
          // Ignore frames we can't parse; the gateway only sends JSON objects.
        }
      },
      onDone: () => _connected.add(false),
      onError: (_) => _connected.add(false),
      cancelOnError: true,
    );
  }

  void _send(Map<String, dynamic> command) {
    _channel?.sink.add(jsonEncode(command));
  }

  /// Send a voice command transcript (also used by the play/pause/next buttons).
  void voice(String transcript) => _send({'type': 'voice', 'transcript': transcript});

  /// Set the navigation destination.
  void setDestination(String destination) =>
      _send({'type': 'set_destination', 'destination': destination});

  /// Change a user setting.
  void setSetting(String key, String value) =>
      _send({'type': 'set_setting', 'key': key, 'value': value});

  /// Close the connection and release resources.
  void dispose() {
    _channel?.sink.close(ws_status.goingAway);
    _events.close();
    _connected.close();
  }
}
