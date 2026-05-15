# ============================================================
# ksnctf #32 Simple Auth - 脆弱性再現環境
# PHP 5.5 (2013年6月リリース) + Apache
# 問題リリース 2013/11/03 当時の環境を再現
# ============================================================
FROM php:5.5-apache

# Apache の設定
RUN a2enmod rewrite

# auth.php を設置
COPY auth.php /var/www/html/auth.php

# ポート80を公開
EXPOSE 80
