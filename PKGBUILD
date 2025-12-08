# Maintainer: Dawood <Ailune@proton.me>
pkgname=tyr-bin
pkgver=0.4.4
pkgrel=1
pkgdesc="Intelligent file organization powered by machine learning and simple algos"
arch=('x86_64' 'aarch64')
url="https://github.com/Aelune/tyr"
license=('GPL-3.0-or-later')
provides=('tyr')
conflicts=('tyr')

source_x86_64=(
  "tyr-linux-x86_64::https://github.com/Aelune/tyr/releases/download/v${pkgver}/tyr-linux-x86_64"
  "LICENSE::https://raw.githubusercontent.com/Aelune/tyr/main/LICENSE"
)
source_aarch64=(
  "tyr-linux-aarch64::https://github.com/Aelune/tyr/releases/download/v${pkgver}/tyr-linux-aarch64"
  "LICENSE::https://raw.githubusercontent.com/Aelune/tyr/main/LICENSE"
)

sha256sums_x86_64=(
    '5466363607637190d13cfca96b8c299e8b9f6d8ef2f1ab609bbdd561d4ed340f'
    'a6ba33e31f75499478db550f25a8239328431ed1697c7bc613f26b84a6366f3f'
    )
sha256sums_aarch64=(
    'e75e7525af5fd0a79055668ac485fbe927aff972fa4b9ae6c1370fbb9bcba5fd'
    'a6ba33e31f75499478db550f25a8239328431ed1697c7bc613f26b84a6366f3f'
    )

package() {
    if [ "$CARCH" = "x86_64" ]; then
        install -Dm755 "$srcdir/tyr-linux-x86_64" "$pkgdir/usr/bin/tyr"
    elif [ "$CARCH" = "aarch64" ]; then
        install -Dm755 "$srcdir/tyr-linux-aarch64" "$pkgdir/usr/bin/tyr"
    fi

    install -Dm644 "$srcdir/LICENSE" "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
