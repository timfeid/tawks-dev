"""A stand-in PuckLab for local work on the PuckLab Online boards.

Serves GET /v1/tawks/leaderboards in the shape the bot expects from PuckLab
(see open/bots/pucklab/README.md) with made-up players linked to the seeded
dev-stack people. GET /v1/account answers for those people as their linked
PuckLab account; every other route answers as an unlinked account would.
It checks no proofs, so it is for local stacks only.

    python3 leaderboards_stub.py 127.0.0.1:18777
"""
import base64
import json
import sys
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def ranked(account, name, user, rating, tier, games, wins, losses, otl, streak, team=None):
    return {'account': account, 'name': name, 'tawks_user_id': user, 'rating': rating, 'tier': tier,
            'games': games, 'wins': wins, 'losses': losses, 'otl': otl, 'streak': streak,
            'best': rating, 'team': {'name': team} if team else None}


BOARDS = {
    'generated_at': 0,
    'ranked': {
        'rosters': [
            {'id': 'tawks.sample', 'name': 'PuckLab League', 'kind': 'league', 'nhl': False, 'players': 6, 'leaders': [
                ranked('a-dazed', 'dazed', 'USR-dazed', 1312, 'PLATINUM', 21, 15, 4, 2, 4, 'Ashmere Harriers'),
                ranked('a-sally', 'sally', 'USR-sally', 1184, 'GOLD', 18, 11, 6, 1, 4, 'Lake Marlow Kodiaks'),
                ranked('a-north', 'northbound', None, 1102, 'GOLD', 12, 7, 5, 0, -2, 'Toronto Maple Leafs'),
                ranked('a-mara', 'mara', 'USR-mara', 1040, 'SILVER', 9, 5, 4, 0, 1),
                ranked('a-pip', 'pip', 'USR-pip', 955, 'SILVER', 7, 3, 4, 0, -1),
                ranked('a-ltfl', 'ltfl', 'USR-ltfl', 990, 'PLACEMENT', 3, 1, 2, 0, -2),
            ],
             'streak_leaders': [{'account': 'a-dazed', 'tawks_user_id': 'USR-dazed', 'streak': 4},
                                {'account': 'a-sally', 'tawks_user_id': 'USR-sally', 'streak': 4}]},
            {'id': 'community.harbor-cup', 'name': 'Harbor Cup', 'kind': 'community', 'nhl': False, 'players': 2, 'leaders': [
                ranked('a-sally', 'sally', 'USR-sally', 1060, 'SILVER', 6, 4, 2, 0, 2),
                ranked('a-pip', 'pip', 'USR-pip', 940, 'SILVER', 6, 2, 4, 0, -3),
            ]},
            {'id': 'nhl-legacy', 'name': 'NHL Legacy 2026', 'kind': 'installed', 'nhl': True, 'players': 0, 'leaders': []},
        ],
        'tiers': [{'name': 'Diamond', 'from': 1500}, {'name': 'Platinum', 'from': 1300}, {'name': 'Gold', 'from': 1100},
                  {'name': 'Silver', 'from': 900}, {'name': 'Bronze', 'from': 0}],
        'placement_games': 5,
    },
    'career': {'enabled': True, 'leagues': [
        {'league': 'pucklab-minor', 'name': 'PuckLab Development League', 'players': 3, 'leaders': [
            {'account': 'a-mara', 'owner': 'mara', 'tawks_user_id': 'USR-mara', 'name': 'Mara Lind', 'position': 'C',
             'team': 'Ashmere Harriers', 'overall': 74, 'age': 19, 'games': 14, 'goals': 9, 'assists': 12, 'points': 21},
            {'account': 'a-north', 'owner': 'northbound', 'tawks_user_id': None, 'name': 'Jonah Reyes', 'position': 'RW',
             'team': 'Boston Bruins', 'overall': 71, 'age': 20, 'games': 13, 'goals': 10, 'assists': 6, 'points': 16},
            {'account': 'a-pip', 'owner': 'pip', 'tawks_user_id': 'USR-pip', 'name': 'Pip Okafor', 'position': 'D',
             'team': 'Lake Marlow Kodiaks', 'overall': 69, 'age': 18, 'games': 12, 'goals': 2, 'assists': 11, 'points': 13},
        ]},
    ]},
    'lab': {'rosters': [
        {'id': 'nhl-legacy', 'name': 'NHL Legacy 2026', 'nhl': True, 'players': 3, 'leaders': [
            {'account': 'a-dazed', 'owner': 'dazed', 'tawks_user_id': 'USR-dazed', 'club': 'Night Shift HC', 'games': 8,
             'wins': 6, 'losses': 1, 'otl': 1, 'goals_for': 31, 'goals_against': 17, 'points': 13},
            {'account': 'a-north', 'owner': 'northbound', 'tawks_user_id': None, 'club': 'Rangers', 'games': 7,
             'wins': 4, 'losses': 3, 'otl': 0, 'goals_for': 22, 'goals_against': 20, 'points': 8},
            {'account': 'a-ltfl', 'owner': 'ltfl', 'tawks_user_id': 'USR-ltfl', 'club': 'Glass Harbor', 'games': 5,
             'wins': 1, 'losses': 4, 'otl': 0, 'goals_for': 11, 'goals_against': 19, 'points': 2},
        ]},
    ]},
}


def linked_accounts():
    accounts = {}
    for section in (BOARDS['ranked']['rosters'], BOARDS['career']['leagues'], BOARDS['lab']['rosters']):
        for board in section:
            for entry in board['leaders']:
                if entry.get('tawks_user_id'):
                    accounts[entry['tawks_user_id']] = (entry['account'], entry.get('owner') or entry['name'])
    return accounts


def subject(header):
    try:
        claims = header.split(' ', 1)[1].split('.', 1)[0]
        return json.loads(base64.urlsafe_b64decode(claims + '=' * (-len(claims) % 4)))['sub']
    except (IndexError, KeyError, ValueError):
        return None


class Handler(BaseHTTPRequestHandler):
    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        path = self.path.split('?')[0]
        if path == '/v1/tawks/leaderboards':
            return self.reply(200, dict(BOARDS, generated_at=int(time.time())))
        linked = linked_accounts().get(subject(self.headers.get('Authorization', '')))
        if path == '/v1/account' and linked:
            return self.reply(200, {'account_id': linked[0], 'name': linked[1], 'access': True, 'secured': True})
        self.reply(403, {'error': 'Connect your PuckLab account with Tawks first.', 'pucklab_link_required': True})

    do_POST = do_GET

    def log_message(self, *args):
        pass


if __name__ == '__main__':
    host, port = (sys.argv[1] if len(sys.argv) > 1 else '127.0.0.1:18777').rsplit(':', 1)
    ThreadingHTTPServer((host, int(port)), Handler).serve_forever()
