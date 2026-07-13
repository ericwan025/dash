import 'dart:async';

import 'package:flutter/material.dart';

import 'gateway_client.dart';

void main() {
  runApp(const DashApp());
}

/// Where the Rust gateway is listening. Matches the gateway's default bind
/// address (`DASH_GATEWAY_ADDR`).
final _gatewayUrl = Uri.parse('ws://127.0.0.1:8080/ws');

class DashApp extends StatelessWidget {
  const DashApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'dash',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true).copyWith(
        scaffoldBackgroundColor: const Color(0xFF0E1116),
        colorScheme: const ColorScheme.dark(
          primary: Color(0xFF4FD1C5),
          surface: Color(0xFF171B22),
        ),
      ),
      home: const DashHome(),
    );
  }
}

class DashHome extends StatefulWidget {
  const DashHome({super.key});

  @override
  State<DashHome> createState() => _DashHomeState();
}

class _DashHomeState extends State<DashHome> {
  late final GatewayClient _client;

  bool _connected = false;
  bool _playing = false;
  String? _track;
  String? _destination;
  final Map<String, String> _settings = {};

  final _destinationController = TextEditingController();
  int _volume = 5;

  StreamSubscription<bool>? _connSub;
  StreamSubscription<Map<String, dynamic>>? _eventSub;

  @override
  void initState() {
    super.initState();
    _client = GatewayClient(_gatewayUrl);
    _connSub = _client.connected.listen((c) {
      if (mounted) setState(() => _connected = c);
    });
    _eventSub = _client.events.listen(_onEvent);
    _client.connect();
  }

  /// Apply a decoded server event to local UI state.
  void _onEvent(Map<String, dynamic> event) {
    if (!mounted) return;
    setState(() {
      switch (event['type']) {
        case 'media_state':
          _playing = event['playing'] == true;
          _track = event['track'] as String?;
          break;
        case 'nav_state':
          _destination = event['destination'] as String?;
          break;
        case 'settings_state':
          final key = event['key'] as String?;
          final value = event['value'] as String?;
          if (key != null && value != null) _settings[key] = value;
          break;
      }
    });
  }

  @override
  void dispose() {
    _connSub?.cancel();
    _eventSub?.cancel();
    _destinationController.dispose();
    _client.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _header(),
              const SizedBox(height: 24),
              _nowPlayingCard(),
              const SizedBox(height: 16),
              _navCard(),
              const SizedBox(height: 16),
              _settingsCard(),
            ],
          ),
        ),
      ),
    );
  }

  Widget _header() {
    return Row(
      children: [
        const Text('dash',
            style: TextStyle(fontSize: 32, fontWeight: FontWeight.w700, letterSpacing: 2)),
        const SizedBox(width: 12),
        const Text('infotainment', style: TextStyle(color: Colors.white38, fontSize: 16)),
        const Spacer(),
        _connectionPill(),
      ],
    );
  }

  Widget _connectionPill() {
    final color = _connected ? const Color(0xFF4FD1C5) : Colors.redAccent;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
      decoration: BoxDecoration(
        color: color.withValues(alpha: 0.15),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(mainAxisSize: MainAxisSize.min, children: [
        Icon(Icons.circle, size: 10, color: color),
        const SizedBox(width: 8),
        Text(_connected ? 'connected' : 'disconnected',
            style: TextStyle(color: color, fontWeight: FontWeight.w600)),
      ]),
    );
  }

  Widget _card({required String title, required Widget child}) {
    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(20),
      decoration: BoxDecoration(
        color: const Color(0xFF171B22),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title.toUpperCase(),
              style: const TextStyle(color: Colors.white38, fontSize: 12, letterSpacing: 1.5)),
          const SizedBox(height: 12),
          child,
        ],
      ),
    );
  }

  Widget _nowPlayingCard() {
    return _card(
      title: 'Media',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_track ?? 'Nothing playing',
              style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w600)),
          const SizedBox(height: 4),
          Text(_playing ? 'Playing' : 'Paused', style: const TextStyle(color: Colors.white54)),
          const SizedBox(height: 16),
          Row(children: [
            _button(Icons.play_arrow, 'Play', () => _client.voice('play music')),
            const SizedBox(width: 12),
            _button(Icons.pause, 'Pause', () => _client.voice('pause')),
            const SizedBox(width: 12),
            _button(Icons.skip_next, 'Next', () => _client.voice('next track')),
          ]),
        ],
      ),
    );
  }

  Widget _navCard() {
    return _card(
      title: 'Navigation',
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(_destination ?? 'No destination set',
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w500)),
          const SizedBox(height: 12),
          Row(children: [
            Expanded(
              child: TextField(
                controller: _destinationController,
                decoration: const InputDecoration(
                  hintText: 'Enter destination',
                  isDense: true,
                  border: OutlineInputBorder(),
                ),
                onSubmitted: _submitDestination,
              ),
            ),
            const SizedBox(width: 12),
            _button(Icons.navigation, 'Go',
                () => _submitDestination(_destinationController.text)),
          ]),
        ],
      ),
    );
  }

  void _submitDestination(String value) {
    final dest = value.trim();
    if (dest.isEmpty) return;
    _client.setDestination(dest);
  }

  Widget _settingsCard() {
    return _card(
      title: 'Settings',
      child: Row(
        children: [
          Expanded(
            child: Text('Volume: ${_settings['volume'] ?? '—'}',
                style: const TextStyle(fontSize: 18)),
          ),
          _button(Icons.volume_up, 'Volume +', () {
            _volume = (_volume % 10) + 1;
            _client.setSetting('volume', _volume.toString());
          }),
        ],
      ),
    );
  }

  Widget _button(IconData icon, String label, VoidCallback onPressed) {
    return FilledButton.tonalIcon(
      onPressed: _connected ? onPressed : null,
      icon: Icon(icon),
      label: Text(label),
    );
  }
}
