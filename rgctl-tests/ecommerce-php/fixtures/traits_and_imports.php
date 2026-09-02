<?php
namespace App\Fixtures;

use App\Traits\Timestampable;
use App\Service\{AuthService, OrderDTO as Order};

trait Loggable
{
    public function log(string $msg): void {}
}

class SampleService
{
    use Timestampable, Loggable;

    public function run(AuthService $auth): void
    {
        AuthService::login('x');
        $obj = null;
        $method = 'ping';
        $obj->$method();
    }
}
